//! Per-tick civilization behaviour (GDD 5.6): each region's six agendas are
//! scored live, and those above the threshold nudge the region. Player boosts
//! decay over time. Deterministic: no RNG.

use crate::data::{Agenda, CivStat, CivilizationBalance, RegionBalance};
use crate::world::{agenda_score, Region, RegionAgendas};

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
        for (i, agenda) in agendas.iter().enumerate() {
            let score = agenda_score(agenda, &regions[idx], entry.boost(i));
            if score >= balance.apply_threshold {
                let (dp, dc, dd, dm) = stat_deltas(agenda.effect_stat, agenda.effect_amount);
                regions[idx].apply_deltas(dp, dc, dd, dm, region_balance);
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
    fn boosting_an_agenda_can_push_it_active() {
        let data = GameData::load().unwrap();
        let world = WorldState::new(&data);
        // A large boost guarantees the first agenda crosses the threshold.
        let region = &world.regions[0];
        let base_score = agenda_score(&data.agendas[0], region, 0.0);
        let boosted = agenda_score(&data.agendas[0], region, 100.0);
        assert!(boosted > base_score);
        assert!(boosted >= data.balance.civilization.apply_threshold);
    }
}
