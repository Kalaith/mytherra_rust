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
    heroes: &mut [Hero],
    regions: &mut [Region],
    balance: &ChampionBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    for champion in champions.iter_mut() {
        let Some(idx) = heroes
            .iter()
            .position(|h| h.id == champion.hero_id && h.is_alive)
        else {
            continue; // dormant while the hero is dead or missing
        };

        champion.quest_progress += champion.quest_step(heroes[idx].level, balance);
        if champion.quest_progress < balance.quest.goal {
            continue;
        }

        champion.quest_progress -= balance.quest.goal;
        champion.quests += 1;
        champion.recompute_rank(balance);

        resolve_rivalry(
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
        // attention carries them toward legend (GDD 5.4 -> the renown web).
        heroes[idx].renown += balance.renown_per_quest;
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

    let (template, prosperity, chaos, danger, magic, strife) = if strength >= threat {
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
}
