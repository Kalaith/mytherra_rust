//! Per-tick civilization behaviour (GDD 5.6): each region pursues its single
//! dominant agenda — the highest-scoring one that clears the threshold — which
//! nudges the region. Player boosts decay over time. Deterministic: no RNG.

use crate::data::{Agenda, CivStat, CivilizationBalance, RegionBalance, SpilloverTarget};
use crate::world::{dominant_agenda, Region, RegionAgendas};

/// Advance every region's agendas by one tick.
pub fn tick_civilization(
    civ: &mut [RegionAgendas],
    regions: &mut [Region],
    agendas: &[Agenda],
    balance: &CivilizationBalance,
    region_balance: &RegionBalance,
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
            let (dp, dc, dd, dm) = stat_deltas(agenda.effect_stat, agenda.effect_amount);
            regions[idx].apply_deltas(dp, dc, dd, dm, region_balance);

            // An outward-facing agenda presses upon a peer — the first time
            // civilizations touch one another: a rivalrous region destabilizes
            // the neighbour it envies, an expansionist one leans on the weakest
            // (GDD 5.6).
            if agenda.spillover_amount != 0.0 {
                if let Some(target) = spillover_target(regions, idx, agenda.spillover_target) {
                    let (sp, sc, sd, sm) =
                        stat_deltas(agenda.spillover_stat, agenda.spillover_amount);
                    regions[target].apply_deltas(sp, sc, sd, sm, region_balance);
                }
            }
        }
    }
}

/// The peer region an outward-facing agenda presses upon, by prosperity. Excludes
/// the acting region; deterministic given a fixed region order.
fn spillover_target(regions: &[Region], self_idx: usize, rule: SpilloverTarget) -> Option<usize> {
    let others = regions.iter().enumerate().filter(|(i, _)| *i != self_idx);
    match rule {
        SpilloverTarget::None => None,
        SpilloverTarget::MostProsperous => others
            .max_by(|(_, a), (_, b)| a.prosperity.total_cmp(&b.prosperity))
            .map(|(i, _)| i),
        SpilloverTarget::LeastProsperous => others
            .min_by(|(_, a), (_, b)| a.prosperity.total_cmp(&b.prosperity))
            .map(|(i, _)| i),
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
    use crate::data::GameData;
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
            &data.balance.civilization,
            &data.balance.region,
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
            &data.balance.civilization,
            &data.balance.region,
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
            spillover_target(&world.regions, 0, SpilloverTarget::MostProsperous),
            Some(1),
            "envy should fall on the strongest *other* region, not the acting one"
        );
        assert_eq!(
            spillover_target(&world.regions, 0, SpilloverTarget::LeastProsperous),
            Some(2)
        );
        assert_eq!(
            spillover_target(&world.regions, 0, SpilloverTarget::None),
            None
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
            &data.balance.civilization,
            &data.balance.region,
        );

        assert!(
            world.regions[2].danger > envied_before,
            "the most prosperous rival should be destabilized by the spillover"
        );
    }
}
