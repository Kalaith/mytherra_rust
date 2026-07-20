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
        let focus = balance.focuses.get(champion.focus);
        let scale = balance.passive_scale * champion.rank as f32;
        if let Some(region) = regions.iter_mut().find(|r| r.id == heroes[idx].region_id) {
            region.apply_deltas(
                focus.resolve_prosperity * scale,
                0.0,
                focus.resolve_danger * scale,
                focus.resolve_magic * scale,
                region_balance,
            );
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
mod tests {
    use super::*;
    use crate::data::{ChampionFocus, GameData};
    use crate::world::WorldState;

    #[test]
    fn a_champions_completed_quest_earns_its_hero_renown() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let hero_id = world.heroes[0].id.clone();
        let before = world.heroes[0].renown;

        let mut champion = Champion::designate(hero_id.clone(), ChampionFocus::Valor);
        champion.quest_progress = data.balance.champion.quest.goal; // completes this tick
        let mut champions = vec![champion];

        tick_champions(
            &mut champions,
            &mut world.heroes,
            &mut world.regions,
            &data.balance.champion,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );

        assert_eq!(champions[0].quests, 1);
        let after = world
            .heroes
            .iter()
            .find(|h| h.id == hero_id)
            .unwrap()
            .renown;
        assert!(
            (after - before - data.balance.champion.renown_per_quest).abs() < 0.001,
            "a completed quest should grant exactly renown_per_quest"
        );
    }

    #[test]
    fn a_champion_is_retired_when_its_hero_dies() {
        // A champion whose hero has fallen is dropped from the roster (freeing a
        // slot for a successor) and its passing is chronicled (GDD 5.4).
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let hero_id = world.heroes[0].id.clone();
        let hero_name = world.heroes[0].name.clone();
        let mut champions = vec![Champion::designate(hero_id, ChampionFocus::Valor)];

        world.heroes[0].is_alive = false; // the hero falls

        tick_champions(
            &mut champions,
            &mut world.heroes,
            &mut world.regions,
            &data.balance.champion,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );

        assert!(
            champions.is_empty(),
            "a dead hero's champion should be retired, freeing the roster slot"
        );
        assert!(
            world
                .chronicle
                .iter_newest()
                .any(|e| e.message.contains(&hero_name)),
            "the champion's passing should be chronicled"
        );
    }

    #[test]
    fn a_champion_passively_guards_its_home() {
        // A cultivated Valor champion holds back its region's danger every tick,
        // even without completing a quest (GDD 5.4).
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let hero = world.heroes[0].clone();
        let region_idx = world
            .regions
            .iter()
            .position(|r| r.id == hero.region_id)
            .unwrap();
        world.regions[region_idx].danger = 60.0;
        let danger_before = world.regions[region_idx].danger;

        let mut champion = Champion::designate(hero.id.clone(), ChampionFocus::Valor);
        champion.rank = 5; // deeply cultivated
        champion.quest_progress = 0.0; // nowhere near completing a quest this tick
        let mut champions = vec![champion];

        tick_champions(
            &mut champions,
            &mut world.heroes,
            &mut world.regions,
            &data.balance.champion,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );

        assert_eq!(
            champions[0].quests, 0,
            "the champion completed no quest this tick"
        );
        assert!(
            world.regions[region_idx].danger < danger_before,
            "a Valor champion's presence should still hold back danger"
        );
    }

    #[test]
    fn strong_champion_calms_its_region() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        // A maxed champion on the calmest region should resolve, not escalate.
        let hero = world.heroes[0].clone();
        let mut champion = Champion::designate(hero.id.clone(), ChampionFocus::Valor);
        champion.bond = 300.0;
        champion.rank = 10;
        champion.quest_progress = data.balance.champion.quest.goal;
        let mut champions = vec![champion];

        let region_idx = world
            .regions
            .iter()
            .position(|r| r.id == hero.region_id)
            .unwrap();
        let danger_before = world.regions[region_idx].danger;

        tick_champions(
            &mut champions,
            &mut world.heroes,
            &mut world.regions,
            &data.balance.champion,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );

        assert_eq!(champions[0].quests, 1);
        assert!(world.regions[region_idx].danger <= danger_before);
    }

    #[test]
    fn focus_shapes_the_resolution_effect() {
        // A Wisdom champion resolving a rivalry kindles its region's magic — an
        // effect a Valor champion (which instead cuts danger) never produces.
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let hero = world.heroes[0].clone();
        let region_idx = world
            .regions
            .iter()
            .position(|r| r.id == hero.region_id)
            .unwrap();
        let magic_before = world.regions[region_idx].magic_affinity;

        let mut champion = Champion::designate(hero.id.clone(), ChampionFocus::Wisdom);
        champion.bond = 300.0;
        champion.rank = 10;
        champion.quest_progress = data.balance.champion.quest.goal;
        let mut champions = vec![champion];

        tick_champions(
            &mut champions,
            &mut world.heroes,
            &mut world.regions,
            &data.balance.champion,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );

        assert!(
            world.regions[region_idx].magic_affinity > magic_before,
            "wisdom focus should kindle magic on a resolved rivalry"
        );
    }

    #[test]
    fn a_resolving_champion_bleeds_secession_pressure() {
        // A strong champion holding a region should quell not just the rivalry
        // but the strife feeding the genesis fracture system (GDD 5.4 ↔ 5.2).
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let hero = world.heroes[0].clone();
        let region_idx = world
            .regions
            .iter()
            .position(|r| r.id == hero.region_id)
            .unwrap();
        world.regions[region_idx].strife = 60.0;

        let mut champion = Champion::designate(hero.id.clone(), ChampionFocus::Devotion);
        champion.bond = 300.0;
        champion.rank = 10;
        champion.quest_progress = data.balance.champion.quest.goal;
        let mut champions = vec![champion];

        tick_champions(
            &mut champions,
            &mut world.heroes,
            &mut world.regions,
            &data.balance.champion,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );

        assert!(
            world.regions[region_idx].strife < 60.0,
            "a resolved rivalry should bleed secession pressure"
        );
    }

    #[test]
    fn a_routed_champion_frays_its_bond() {
        // A modest champion sent against an overwhelming region is defeated, and
        // pays for it: the bond the player cultivated frays (GDD 5.4).
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let hero = world.heroes[0].clone();
        let region_idx = world
            .regions
            .iter()
            .position(|r| r.id == hero.region_id)
            .unwrap();
        world.regions[region_idx].danger = 100.0;
        world.regions[region_idx].chaos = 100.0;
        world.regions[region_idx].strife = 100.0;

        let mut champion = Champion::designate(hero.id.clone(), ChampionFocus::Valor);
        champion.bond = 50.0;
        champion.quest_progress = data.balance.champion.quest.goal;
        let mut champions = vec![champion];

        tick_champions(
            &mut champions,
            &mut world.heroes,
            &mut world.regions,
            &data.balance.champion,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );

        assert_eq!(champions[0].quests, 1);
        assert!(
            champions[0].bond < 50.0,
            "a routed champion should fray its bond"
        );
        assert!(champions[0].bond >= 0.0, "bond never goes negative");
    }

    #[test]
    fn a_harder_won_triumph_forges_more_renown() {
        // Two triumphs by the same champion, differing only in the region's
        // threat: quelling the dangerous land forges more renown (GDD 5.4).
        let data = GameData::load().unwrap();
        let renown_gained = |danger: f32, chaos: f32| {
            let mut world = WorldState::new(&data);
            let hero = world.heroes[0].clone();
            let region_idx = world
                .regions
                .iter()
                .position(|r| r.id == hero.region_id)
                .unwrap();
            world.regions[region_idx].danger = danger;
            world.regions[region_idx].chaos = chaos;
            let before = world.heroes[0].renown;

            let mut champion = Champion::designate(hero.id.clone(), ChampionFocus::Valor);
            champion.bond = 500.0; // strong enough to triumph in both regions
            champion.rank = 10;
            champion.quest_progress = data.balance.champion.quest.goal;
            let mut champions = vec![champion];

            tick_champions(
                &mut champions,
                &mut world.heroes,
                &mut world.regions,
                &data.balance.champion,
                &data.balance.region,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
            assert_eq!(champions[0].quests, 1);
            world.heroes[0].renown - before
        };

        let calm = renown_gained(10.0, 10.0);
        let dangerous = renown_gained(80.0, 80.0);
        assert!(
            dangerous > calm,
            "quelling a dangerous region should forge more renown than a quiet one"
        );
    }
}
