//! Per-tick settlement growth (GDD 5.3): population grows on the settlement's
//! and its region's prosperity; settlement prosperity tracks its region (raised
//! by its buildings); and a thriving settlement feeds prosperity back to that
//! region (one of the region "pressure" terms §5.2 left stubbed until now).
//! Prosperous, populous settlements also raise new buildings over time (GDD 6),
//! the one settlement effect that draws on the world RNG.

use crate::data::strings::ChronicleText;
use crate::data::{fill, BuildingType, RegionBalance, SettlementBalance};
use crate::world::{Building, Chronicle, EventKind, Region, Settlement};
use macroquad_toolkit::data_loader::DataRegistry;
use macroquad_toolkit::math::approach;
use macroquad_toolkit::rng::SeededRng;

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

/// Prosperous, populous settlements raise new buildings over time (GDD 6). A
/// settlement holds at most one of each building type; the chosen type is drawn
/// deterministically from the world RNG (candidates sorted for determinism, as
/// the type registry is a hash map).
#[allow(clippy::too_many_arguments)]
pub fn tick_construction(
    settlements: &[Settlement],
    buildings: &mut Vec<Building>,
    building_types: &DataRegistry<BuildingType>,
    balance: &SettlementBalance,
    rng: &mut SeededRng,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    for settlement in settlements {
        if settlement.prosperity < balance.construction_prosperity_min
            || settlement.population < balance.construction_population_min
        {
            continue;
        }
        if !rng.chance(balance.construction_chance) {
            continue;
        }

        // Building types this settlement doesn't already have, sorted by id so
        // the RNG draw is reproducible regardless of hash-map iteration order.
        let mut candidates: Vec<&BuildingType> = building_types
            .iter()
            .map(|(_, t)| t)
            .filter(|t| {
                !buildings
                    .iter()
                    .any(|b| b.settlement_id == settlement.id && b.type_id == t.id)
            })
            .collect();
        candidates.sort_by(|a, b| a.id.cmp(&b.id));
        let Some(&chosen) = rng.choose(&candidates) else {
            continue;
        };

        buildings.push(Building {
            id: format!("{}_{}", settlement.id, chosen.id),
            name: format!("{} {}", settlement.name, chosen.name),
            settlement_id: settlement.id.clone(),
            type_id: chosen.id.clone(),
            prosperity_bonus: chosen.prosperity_bonus,
        });
        chronicle.push(
            year,
            EventKind::Region,
            fill(
                &text.settlement_built,
                &[
                    ("settlement", settlement.name.clone()),
                    ("building", chosen.name.clone()),
                ],
            ),
        );
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

    #[test]
    fn thriving_settlements_construct_new_buildings() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let before = world.buildings.len();
        // Pin every settlement well above the construction floors, then run long
        // enough that the per-tick chance fires.
        for settlement in &mut world.settlements {
            settlement.prosperity = 90.0;
            settlement.population = 10_000.0;
        }
        for _ in 0..400 {
            tick_construction(
                &world.settlements,
                &mut world.buildings,
                &data.building_types,
                &data.balance.settlement,
                &mut world.rng,
                &mut world.chronicle,
                &data.strings.chronicle,
                1,
            );
        }
        assert!(
            world.buildings.len() > before,
            "thriving settlements should have raised buildings: {} -> {}",
            before,
            world.buildings.len()
        );
        // No settlement ends up with two of the same building type.
        for settlement in &world.settlements {
            let mut types: Vec<&str> = world
                .buildings
                .iter()
                .filter(|b| b.settlement_id == settlement.id)
                .map(|b| b.type_id.as_str())
                .collect();
            let total = types.len();
            types.sort_unstable();
            types.dedup();
            assert_eq!(
                total,
                types.len(),
                "duplicate building type in a settlement"
            );
        }
    }
}
