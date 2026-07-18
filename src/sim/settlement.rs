//! Per-tick settlement growth (GDD 5.3): population grows on the settlement's
//! and its region's prosperity; settlement prosperity tracks its region; and a
//! thriving settlement feeds prosperity back to that region (one of the region
//! "pressure" terms §5.2 left stubbed until now). Deterministic: no RNG.

use crate::data::{RegionBalance, SettlementBalance};
use crate::world::{Building, Region, Settlement};
use macroquad_toolkit::math::approach;

pub fn tick_settlements(
    settlements: &mut [Settlement],
    buildings: &[Building],
    regions: &mut [Region],
    balance: &SettlementBalance,
    region_balance: &RegionBalance,
) {
    for settlement in settlements.iter_mut() {
        let Some(idx) = regions.iter().position(|r| r.id == settlement.region_id) else {
            continue;
        };
        let region_prosperity = regions[idx].prosperity;
        let region_chaos = regions[idx].chaos;

        // Buildings raise the settlement's prosperity equilibrium.
        let building_bonus: f32 = buildings
            .iter()
            .filter(|b| b.settlement_id == settlement.id)
            .map(|b| b.prosperity_bonus)
            .sum();
        let target = (region_prosperity + building_bonus).clamp(0.0, 100.0);

        let growth = settlement.growth_rate(region_prosperity, region_chaos, balance);
        settlement.population = (settlement.population * (1.0 + growth)).max(0.0);
        settlement.prosperity =
            approach(settlement.prosperity, target, balance.prosperity_drift_rate)
                .clamp(0.0, 100.0);

        let contribution = settlement.region_contribution(balance);
        regions[idx].apply_deltas(contribution, 0.0, 0.0, 0.0, region_balance);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::world::WorldState;

    #[test]
    fn calm_prosperous_region_grows_its_settlements() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        for region in &mut world.regions {
            region.prosperity = 80.0;
            region.chaos = 10.0;
        }
        let before: Vec<f32> = world.settlements.iter().map(|s| s.population).collect();
        tick_settlements(
            &mut world.settlements,
            &world.buildings,
            &mut world.regions,
            &data.balance.settlement,
            &data.balance.region,
        );
        for (s, was) in world.settlements.iter().zip(before) {
            assert!(s.population > was);
        }
    }

    #[test]
    fn buildings_lift_settlement_prosperity_above_its_region() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        // Hold every region at a fixed prosperity so only the building bonus
        // (Aldervale = Market 6 + Granary 4 = 10) moves the settlement's target.
        for _ in 0..40 {
            for region in &mut world.regions {
                region.prosperity = 50.0;
            }
            tick_settlements(
                &mut world.settlements,
                &world.buildings,
                &mut world.regions,
                &data.balance.settlement,
                &data.balance.region,
            );
        }
        let aldervale = world
            .settlements
            .iter()
            .find(|s| s.id == "aldervale")
            .unwrap();
        assert!(
            aldervale.prosperity > 55.0,
            "buildings should lift prosperity above the region baseline: {}",
            aldervale.prosperity
        );
    }
}
