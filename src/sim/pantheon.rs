//! Per-tick pantheon behaviour (GDD 5.6): each deity's pressure drifts toward a
//! baseline, and a roused deity presses its domain upon every region scaled by
//! its pressure tier. Deterministic: no RNG.

use crate::data::{PantheonBalance, PantheonStat, RegionBalance};
use crate::world::{PantheonDeity, Region};
use macroquad_toolkit::math::approach;

/// Advance every deity by one tick and apply their domain pressure.
pub fn tick_pantheon(
    deities: &mut [PantheonDeity],
    regions: &mut [Region],
    balance: &PantheonBalance,
    region_balance: &RegionBalance,
) {
    for deity in deities.iter_mut() {
        deity.cooldown = (deity.cooldown - 1).max(0);
        deity.pressure = approach(deity.pressure, balance.drift_target, balance.drift_rate);

        let scale = deity.tier_multiplier(balance);
        if scale > 0.0 {
            let (dp, dc, dd, dm) = stat_deltas(deity.effect_stat, deity.effect_amount * scale);
            for region in regions.iter_mut() {
                region.apply_deltas(dp, dc, dd, dm, region_balance);
            }
        }
    }
}

/// Map a pantheon stat + amount onto (prosperity, chaos, danger, magic) deltas.
fn stat_deltas(stat: PantheonStat, amount: f32) -> (f32, f32, f32, f32) {
    match stat {
        PantheonStat::Prosperity => (amount, 0.0, 0.0, 0.0),
        PantheonStat::Chaos => (0.0, amount, 0.0, 0.0),
        PantheonStat::Danger => (0.0, 0.0, amount, 0.0),
        PantheonStat::Magic => (0.0, 0.0, 0.0, amount),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::world::WorldState;

    #[test]
    fn pressure_drifts_toward_baseline() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        world.pantheon[0].pressure = 95.0;
        tick_pantheon(
            &mut world.pantheon,
            &mut world.regions,
            &data.balance.pantheon,
            &data.balance.region,
        );
        assert!(world.pantheon[0].pressure < 95.0);
    }

    #[test]
    fn roused_deity_presses_its_domain() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        // Aurex (prosperity) at full pressure should raise prosperity.
        let idx = world.pantheon.iter().position(|d| d.id == "aurex").unwrap();
        world.pantheon[idx].pressure = 100.0;
        let before = world.regions[0].prosperity;
        tick_pantheon(
            &mut world.pantheon,
            &mut world.regions,
            &data.balance.pantheon,
            &data.balance.region,
        );
        assert!(world.regions[0].prosperity >= before);
    }

    #[test]
    fn every_ally_and_rival_id_resolves() {
        // The ally/rival web is hand-wired; a typo would silently render as a raw
        // id in the UI. Guard that every reference points at a real deity.
        let data = GameData::load().unwrap();
        let world = WorldState::new(&data);
        let ids: Vec<&str> = world.pantheon.iter().map(|d| d.id.as_str()).collect();
        for deity in &world.pantheon {
            assert!(
                ids.contains(&deity.ally_id.as_str()),
                "{} has unknown ally {}",
                deity.id,
                deity.ally_id
            );
            assert!(
                ids.contains(&deity.rival_id.as_str()),
                "{} has unknown rival {}",
                deity.id,
                deity.rival_id
            );
            assert_ne!(deity.ally_id, deity.id, "{} allies itself", deity.id);
            assert_ne!(deity.rival_id, deity.id, "{} rivals itself", deity.id);
        }
    }
}
