//! Per-tick champion advancement: quest progress and deterministic rivalry
//! resolution when a quest completes (GDD 5.4). No RNG — rivalries are decided
//! by strength vs. threat, not a dice roll.

use crate::data::strings::ChronicleText;
use crate::data::{fill, ChampionBalance, RegionBalance};
use crate::world::{Champion, Chronicle, EventKind, Hero, Region};

/// Advance every champion whose hero is alive by one tick.
#[allow(clippy::too_many_arguments)]
pub fn tick_champions(
    champions: &mut [Champion],
    heroes: &[Hero],
    regions: &mut [Region],
    balance: &ChampionBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    for champion in champions.iter_mut() {
        let Some(hero) = heroes
            .iter()
            .find(|h| h.id == champion.hero_id && h.is_alive)
        else {
            continue; // dormant while the hero is dead or missing
        };

        champion.quest_progress += champion.quest_step(hero.level, balance);
        if champion.quest_progress < balance.quest.goal {
            continue;
        }

        champion.quest_progress -= balance.quest.goal;
        champion.quests += 1;
        champion.recompute_rank(balance);

        resolve_rivalry(
            champion,
            hero,
            regions,
            balance,
            region_balance,
            chronicle,
            text,
            year,
        );
    }
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
) {
    let Some(region) = regions.iter_mut().find(|r| r.id == hero.region_id) else {
        return;
    };
    let r = &balance.rivalry;
    let strength = champion.bond * r.strength_bond
        + champion.rank as f32 * r.strength_rank
        + hero.level as f32 * r.strength_level;
    let threat =
        region.pressure() + region.danger * r.threat_danger + region.chaos / r.threat_chaos_div;

    let (template, prosperity, chaos, danger) = if strength >= threat {
        (
            &text.champion_resolved,
            r.resolved_prosperity,
            r.resolved_chaos,
            r.resolved_danger,
        )
    } else {
        (
            &text.champion_escalated,
            0.0,
            r.escalated_chaos,
            r.escalated_danger,
        )
    };
    region.apply_deltas(prosperity, chaos, danger, region_balance);
    chronicle.push(
        year,
        EventKind::Hero,
        fill(
            template,
            &[("hero", hero.name.clone()), ("region", region.name.clone())],
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{ChampionFocus, GameData};
    use crate::world::WorldState;

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
            &world.heroes,
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
}
