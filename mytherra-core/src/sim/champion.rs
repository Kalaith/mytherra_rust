//! Per-tick champion advancement: quest progress and deterministic rivalry
//! resolution when a quest completes (GDD 5.4). No RNG — rivalries are decided
//! by strength vs. threat, not a dice roll.

use crate::data::strings::ChronicleText;
use crate::data::{fill, ChampionBalance, RegionBalance};
use crate::world::{Champion, Chronicle, EventKind, Hero, Region};

/// Advance every champion whose hero is alive by one tick.
#[allow(clippy::too_many_arguments)]
pub fn tick_champions(
    champions: &mut Vec<Champion>,
    heroes: &mut [Hero],
    regions: &mut [Region],
    balance: &ChampionBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    // The patron-bond ends with the hero: a champion whose hero has died (or
    // passed from the world) is retired at once, freeing its roster slot so the
    // player can raise a successor, and its passing is marked — the close of an
    // arc the player invested favor to build (GDD 5.4).
    champions.retain(|champion| {
        let living = heroes
            .iter()
            .any(|h| h.id == champion.hero_id && h.is_alive);
        if !living {
            let name = heroes
                .iter()
                .find(|h| h.id == champion.hero_id)
                .map(|h| h.name.clone())
                .unwrap_or_else(|| champion.hero_id.clone());
            chronicle.push(
                year,
                EventKind::Hero,
                fill(&text.champion_retired, &[("hero", name)]),
            );
        }
        living
    });

    for champion in champions.iter_mut() {
        let Some(idx) = heroes
            .iter()
            .position(|h| h.id == champion.hero_id && h.is_alive)
        else {
            continue; // defensive: retirement above already dropped these
        };

        // A cultivated champion continuously shapes its home by its focus (GDD
        // 5.4) — a Valor champion holds back danger, Wisdom kindles magic,
        // Devotion lifts prosperity — scaled by rank, so a deeper investment
        // guards or enriches the land more, every tick, not just at resolution.
        // A focus that suits the hero's own nature (Valor for a warrior, Devotion
        // for a cleric) shapes the land more strongly still, so matching focus to
        // role rewards the player who cultivates along the grain (GDD 5.4).
        let focus = balance.focuses.get(champion.focus);
        let synergy = if champion.focus.suits(heroes[idx].role) {
            1.0 + balance.focus_synergy_bonus
        } else {
            1.0
        };
        let scale = balance.passive_scale * champion.rank as f32 * synergy;
        if let Some(region) = regions.iter_mut().find(|r| r.id == heroes[idx].region_id) {
            region.apply_deltas(
                focus.resolve_prosperity * scale,
                0.0,
                focus.resolve_danger * scale,
                focus.resolve_magic * scale,
                region_balance,
            );
            // A beloved champion holds their homeland together just by dwelling in
            // it: their presence continuously bleeds off the secession pressure
            // that would fracture the region (GDD 5.4 <-> 5.2), scaled by rank —
            // so cultivating a champion is a standing guard against fracture, the
            // mirror of the shield a strong hero gives against conquest.
            region.adjust_strife(-balance.passive_strife * champion.rank as f32);
        }

        champion.quest_progress += champion.quest_step(heroes[idx].level, balance);
        if champion.quest_progress < balance.quest.goal {
            continue;
        }

        champion.quest_progress -= balance.quest.goal;
        champion.quests += 1;
        champion.recompute_rank(balance);

        let outcome = resolve_rivalry(
            champion,
            &heroes[idx],
            regions,
            balance,
            region_balance,
            chronicle,
            text,
            year,
        );
        // A completed quest is a deed that spreads the champion's fame; a patron's
        // attention carries them toward legend (GDD 5.4 -> the renown web). A
        // triumph over a dangerous region forges more legend than a quiet one,
        // while a rout frays the bond the player paid to build.
        let r = &balance.rivalry;
        if outcome.resolved {
            heroes[idx].renown +=
                balance.renown_per_quest + outcome.threat * r.triumph_renown_per_threat;
        } else {
            heroes[idx].renown += balance.renown_per_quest;
            champion.bond = (champion.bond - outcome.shortfall * r.defeat_bond_loss).max(0.0);
        }
    }
}

/// The margin of a resolved rivalry: whether the champion prevailed, and the
/// figures the caller scales its reward or setback by.
struct RivalryOutcome {
    resolved: bool,
    /// Threat the champion faced (used to scale a triumph's renown).
    threat: f32,
    /// How far strength fell short of threat on a defeat, else 0.
    shortfall: f32,
}

/// Resolve a quest's rivalry against the hero's current region.
#[allow(clippy::too_many_arguments)]
fn resolve_rivalry(
    champion: &Champion,
    hero: &Hero,
    regions: &mut [Region],
    balance: &ChampionBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) -> RivalryOutcome {
    let Some(region) = regions.iter_mut().find(|r| r.id == hero.region_id) else {
        // No region to contest: treat as a bloodless draw — no reward, no setback.
        return RivalryOutcome {
            resolved: false,
            threat: 0.0,
            shortfall: 0.0,
        };
    };
    let r = &balance.rivalry;
    let strength = champion.bond * r.strength_bond
        + champion.rank as f32 * r.strength_rank
        + hero.level as f32 * r.strength_level;
    let threat =
        region.pressure() + region.danger * r.threat_danger + region.chaos / r.threat_chaos_div;
    let resolved = strength >= threat;

    let (template, prosperity, chaos, danger, magic, strife) = if resolved {
        // A successful champion also stamps its focus on the region: Valor cuts
        // danger, Wisdom kindles magic, Devotion lifts prosperity. It further
        // holds the region together, bleeding off secession pressure.
        let focus = balance.focuses.get(champion.focus);
        (
            &text.champion_resolved,
            r.resolved_prosperity + focus.resolve_prosperity,
            r.resolved_chaos,
            r.resolved_danger + focus.resolve_danger,
            focus.resolve_magic,
            r.resolved_strife,
        )
    } else {
        // A defeated champion emboldens unrest, feeding secession pressure.
        (
            &text.champion_escalated,
            0.0,
            r.escalated_chaos,
            r.escalated_danger,
            0.0,
            r.escalated_strife,
        )
    };
    region.apply_deltas(prosperity, chaos, danger, magic, region_balance);
    region.adjust_strife(strife);
    chronicle.push(
        year,
        EventKind::Hero,
        fill(
            template,
            &[("hero", hero.name.clone()), ("region", region.name.clone())],
        ),
    );

    RivalryOutcome {
        resolved,
        threat,
        shortfall: (threat - strength).max(0.0),
    }
}

#[cfg(test)]
mod tests;
