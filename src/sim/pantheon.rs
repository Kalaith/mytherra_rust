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
        // A deity stirs toward a baseline shifted by how ascendant its domain is
        // across the world, so the state of the world rouses the gods.
        let domain = domain_average(regions, deity.effect_stat);
        let target =
            (balance.drift_target + (domain - 50.0) * balance.domain_response).clamp(0.0, 100.0);
        deity.pressure = approach(deity.pressure, target, balance.drift_rate);

        let scale = deity.tier_multiplier(balance);
        if scale > 0.0 {
            let (dp, dc, dd, dm) = stat_deltas(deity.effect_stat, deity.effect_amount * scale);
            for region in regions.iter_mut() {
                region.apply_deltas(dp, dc, dd, dm, region_balance);
            }
        }
    }
}

/// The world's average value of the stat a deity holds domain over.
fn domain_average(regions: &[Region], stat: PantheonStat) -> f32 {
    if regions.is_empty() {
        return 50.0;
    }
    let sum: f32 = regions
        .iter()
        .map(|r| match stat {
            PantheonStat::Prosperity => r.prosperity,
            PantheonStat::Chaos => r.chaos,
            PantheonStat::Danger => r.danger,
            PantheonStat::Magic => r.magic_affinity,
        })
        .sum();
    sum / regions.len() as f32
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
    fn an_ascendant_domain_rouses_its_deity() {
        let data = GameData::load().unwrap();
        let baseline = data.balance.pantheon.drift_target;

        // Mordath holds domain over danger. A world steeped in danger should pull
        // its pressure above the calm baseline...
        let mut dangerous = WorldState::new(&data);
        let idx = dangerous
            .pantheon
            .iter()
            .position(|d| d.effect_stat == PantheonStat::Danger)
            .unwrap();
        for r in &mut dangerous.regions {
            r.danger = 95.0;
        }
        dangerous.pantheon[idx].pressure = baseline;
        tick_pantheon(
            &mut dangerous.pantheon,
            &mut dangerous.regions,
            &data.balance.pantheon,
            &data.balance.region,
        );

        // ...while a placid world lets it settle back down.
        let mut calm = WorldState::new(&data);
        for r in &mut calm.regions {
            r.danger = 5.0;
        }
        calm.pantheon[idx].pressure = baseline;
        tick_pantheon(
            &mut calm.pantheon,
            &mut calm.regions,
            &data.balance.pantheon,
            &data.balance.region,
        );

        assert!(dangerous.pantheon[idx].pressure > baseline);
        assert!(calm.pantheon[idx].pressure < baseline);
        assert!(dangerous.pantheon[idx].pressure > calm.pantheon[idx].pressure);
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
