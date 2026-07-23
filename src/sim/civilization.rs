//! Per-tick civilization behaviour (GDD 5.6): each region pursues its single
//! dominant agenda — the highest-scoring one that clears the threshold — which
//! nudges the region. Player boosts decay over time. Deterministic: no RNG.

use crate::data::strings::ChronicleText;
use crate::data::{fill, Agenda, CivStat, CivilizationBalance, RegionBalance};
use crate::world::{
    dominant_agenda, spillover_target, Chronicle, EventKind, Pact, Region, RegionAgendas, Vassalage,
};

/// Advance every region's agendas by one tick.
#[allow(clippy::too_many_arguments)]
pub fn tick_civilization(
    civ: &mut [RegionAgendas],
    regions: &mut [Region],
    agendas: &[Agenda],
    pacts: &[Pact],
    vassalages: &[Vassalage],
    balance: &CivilizationBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    for entry in civ.iter_mut() {
        entry.cooldown = (entry.cooldown - 1).max(0);
        for boost in entry.boosts.iter_mut() {
            *boost = (*boost - balance.boost_decay).max(0.0);
        }

        let Some(idx) = regions.iter().position(|r| r.id == entry.region_id) else {
            continue;
        };
        if let Some(a) = dominant_agenda(agendas, &regions[idx], entry, balance.apply_threshold) {
            let agenda = &agendas[a];

            // A change of prevailing course is a moment in the region's history:
            // chronicle it once when a *different* agenda takes hold (whether the
            // world's drift or the player's boost redirected it), so the
            // civilization system reads in the chronicle instead of nudging in
            // silence (GDD 5.6). Lapses back to no dominant course leave the last
            // course recorded, so re-adopting it isn't re-announced.
            if entry.current_agenda.as_deref() != Some(agenda.id.as_str()) {
                entry.current_agenda = Some(agenda.id.clone());
                chronicle.push(
                    year,
                    EventKind::Region,
                    fill(
                        &text.agenda_shift,
                        &[
                            ("region", regions[idx].name.clone()),
                            ("agenda", agenda.name.clone()),
                        ],
                    ),
                );
            }

            let (dp, dc, dd, dm) = stat_deltas(agenda.effect_stat, agenda.effect_amount);
            regions[idx].apply_deltas(dp, dc, dd, dm, region_balance);

            // An outward-facing agenda presses upon a peer — the first time
            // civilizations touch one another: a rivalrous region destabilizes
            // the neighbour it envies, an expansionist one leans on the weakest
            // (GDD 5.6).
            if agenda.spillover_amount != 0.0 {
                if let Some(target) =
                    spillover_target(regions, idx, agenda.spillover_target, pacts, vassalages)
                {
                    let (sp, sc, sd, sm) =
                        stat_deltas(agenda.spillover_stat, agenda.spillover_amount);
                    regions[target].apply_deltas(sp, sc, sd, sm, region_balance);
                }
            }
        }
    }
}

/// Map an agenda stat + amount onto (prosperity, chaos, danger, magic) deltas.
fn stat_deltas(stat: CivStat, amount: f32) -> (f32, f32, f32, f32) {
    match stat {
        CivStat::Prosperity => (amount, 0.0, 0.0, 0.0),
        CivStat::Chaos => (0.0, amount, 0.0, 0.0),
        CivStat::Danger => (0.0, 0.0, amount, 0.0),
        CivStat::Magic => (0.0, 0.0, 0.0, amount),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{GameData, SpilloverTarget};
    use crate::world::WorldState;

    #[test]
    fn boosts_decay_each_tick() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        world.civilization[0].boosts[0] = 20.0;
        tick_civilization(
            &mut world.civilization,
            &mut world.regions,
            &data.agendas,
            &[],
            &[],
            &data.balance.civilization,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        assert!(world.civilization[0].boosts[0] < 20.0);
    }

    #[test]
    fn a_boost_makes_an_agenda_the_regions_dominant_course() {
        let data = GameData::load().unwrap();
        let world = WorldState::new(&data);
        let region = &world.regions[0];
        let threshold = data.balance.civilization.apply_threshold;

        // Massively boosting one agenda makes it the region's dominant course,
        // regardless of which one naturally led.
        let mut entry = RegionAgendas::new(region.id.clone(), data.agendas.len());
        let target = data.agendas.len() - 1;
        entry.boosts[target] = 500.0;
        assert_eq!(
            dominant_agenda(&data.agendas, region, &entry, threshold),
            Some(target)
        );
    }

    #[test]
    fn only_the_dominant_agenda_applies_its_effect() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        // Force Rivalry (raises danger) to dominate region 0.
        let rivalry = data.agendas.iter().position(|a| a.id == "rivalry").unwrap();
        world.civilization[0].boosts[rivalry] = 500.0;
        let danger_before = world.regions[0].danger;

        tick_civilization(
            &mut world.civilization,
            &mut world.regions,
            &data.agendas,
            &[],
            &[],
            &data.balance.civilization,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );

        assert!(
            world.regions[0].danger > danger_before,
            "the dominant Rivalry agenda should raise danger"
        );
    }

    #[test]
    fn spillover_targets_the_right_peer_by_prosperity() {
        // The selection is by prosperity and always excludes the acting region,
        // so a rival envies the strongest peer and an expansionist leans on the
        // weakest — never itself (GDD 5.6).
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        world.regions[0].prosperity = 99.0; // acting region: the strongest overall
        world.regions[1].prosperity = 70.0;
        world.regions[2].prosperity = 40.0;
        world.regions[3].prosperity = 55.0;

        assert_eq!(
            spillover_target(&world.regions, 0, SpilloverTarget::MostProsperous, &[], &[]),
            Some(1),
            "envy should fall on the strongest *other* region, not the acting one"
        );
        assert_eq!(
            spillover_target(
                &world.regions,
                0,
                SpilloverTarget::LeastProsperous,
                &[],
                &[]
            ),
            Some(2)
        );
        assert_eq!(
            spillover_target(&world.regions, 0, SpilloverTarget::None, &[], &[]),
            None
        );

        // A sworn ally is spared: with region 1 (the richest) allied to the actor,
        // envy falls on the next-richest instead (GDD 5.6 <-> 5.2).
        let pacts = vec![crate::world::Pact {
            id: "p".to_owned(),
            region_a: world.regions[0].id.clone(),
            region_b: world.regions[1].id.clone(),
            age: 1,
        }];
        assert_eq!(
            spillover_target(
                &world.regions,
                0,
                SpilloverTarget::MostProsperous,
                &pacts,
                &[]
            ),
            Some(3),
            "a rivalrous people does not destabilize its own ally"
        );

        // A vassalage bond is spared the same way: with region 1 held by the actor
        // as its vassal, envy again falls on the next-richest instead — an overlord
        // does not press upon the vassal it protects (GDD 5.6 <-> 5.2).
        let vassalages = vec![crate::world::Vassalage {
            id: "v".to_owned(),
            overlord_id: world.regions[0].id.clone(),
            vassal_id: world.regions[1].id.clone(),
            age: 1,
        }];
        assert_eq!(
            spillover_target(
                &world.regions,
                0,
                SpilloverTarget::MostProsperous,
                &[],
                &vassalages
            ),
            Some(3),
            "an overlord does not destabilize its own vassal"
        );
    }

    #[test]
    fn a_change_of_course_is_chronicled_once() {
        // Boosting Rivalry to dominance sets the region's course and chronicles
        // it; a second tick under the same course adds no new line (GDD 5.6).
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let rivalry = data.agendas.iter().position(|a| a.id == "rivalry").unwrap();
        let rivalry_name = data.agendas[rivalry].name.clone();
        world.civilization[0].boosts[rivalry] = 500.0;

        let tick = |world: &mut WorldState| {
            tick_civilization(
                &mut world.civilization,
                &mut world.regions,
                &data.agendas,
                &[],
                &[],
                &data.balance.civilization,
                &data.balance.region,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
        };

        tick(&mut world);
        assert_eq!(
            world.civilization[0].current_agenda.as_deref(),
            Some("rivalry"),
            "the region should have taken up the Rivalry course"
        );
        let shifts = |world: &WorldState| {
            world
                .chronicle
                .iter_newest()
                .filter(|e| {
                    e.message.contains(&rivalry_name) && e.message.contains("sets its course")
                })
                .count()
        };
        assert_eq!(shifts(&world), 1, "the change of course is chronicled once");

        // Re-boost so it stays dominant, then tick again: no second announcement.
        world.civilization[0].boosts[rivalry] = 500.0;
        tick(&mut world);
        assert_eq!(
            shifts(&world),
            1,
            "holding the same course should not re-announce it"
        );
    }

    #[test]
    fn a_rivalrous_region_destabilizes_the_neighbour_it_envies() {
        // Rivalry now reaches beyond its own borders, pressing danger onto the
        // most prosperous *other* region (GDD 5.6).
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let rivalry = data.agendas.iter().position(|a| a.id == "rivalry").unwrap();
        world.civilization[0].boosts[rivalry] = 500.0;
        world.regions[2].prosperity = 95.0; // the clear envy of the realm
        let envied_before = world.regions[2].danger;

        tick_civilization(
            &mut world.civilization,
            &mut world.regions,
            &data.agendas,
            &[],
            &[],
            &data.balance.civilization,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );

        assert!(
            world.regions[2].danger > envied_before,
            "the most prosperous rival should be destabilized by the spillover"
        );
    }
}
