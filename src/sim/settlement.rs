//! Per-tick settlement growth (GDD 5.3): population grows on the settlement's
//! and its region's prosperity; settlement prosperity tracks its region (raised
//! by its buildings); and a thriving settlement feeds prosperity back to that
//! region (one of the region "pressure" terms §5.2 left stubbed until now).
//! Prosperous, populous settlements also raise new buildings over time (GDD 6),
//! the one settlement effect that draws on the world RNG.

use crate::data::strings::ChronicleText;
use crate::data::{
    fill, BuildingType, Culture, RegionBalance, ResourceStatus, SettlementBalance,
    SettlementNameBank, SettlementSeed,
};
use crate::world::{Building, Chronicle, EventKind, Region, ResourceNode, Settlement};
use macroquad_toolkit::data_loader::DataRegistry;
use macroquad_toolkit::math::approach;
use macroquad_toolkit::rng::SeededRng;

#[allow(clippy::too_many_arguments)]
pub fn tick_settlements(
    settlements: &mut [Settlement],
    buildings: &[Building],
    regions: &mut [Region],
    resource_nodes: &[ResourceNode],
    balance: &SettlementBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    tier_names: &[String],
    year: u32,
) {
    for settlement in settlements.iter_mut() {
        let Some(idx) = regions.iter().position(|r| r.id == settlement.region_id) else {
            continue;
        };
        let region_prosperity = regions[idx].prosperity;
        let region_chaos = regions[idx].chaos;

        // Buildings raise the settlement's prosperity equilibrium — and one whose
        // trade draws on a resource its region actually produces earns an extra
        // bonus, so a Forge over ore or a Harbor over a fishery pays off more than
        // the same building raised over barren ground (GDD 6 <-> 5.3).
        let building_bonus: f32 = buildings
            .iter()
            .filter(|b| b.settlement_id == settlement.id)
            .map(|b| {
                let synergy = b
                    .synergy_resource
                    .is_some_and(|res| region_produces(resource_nodes, &settlement.region_id, res));
                b.prosperity_bonus
                    + if synergy {
                        balance.building_synergy_bonus
                    } else {
                        0.0
                    }
            })
            .sum();
        let supporting = (region_prosperity + building_bonus).clamp(0.0, 100.0);

        // A settlement's houses of worship hallow the land around them (GDD 6 <->
        // 5.1): every Temple raises its region's divine resonance a little each
        // tick — a built path to faith beside a Cleric's tending and the player's
        // consecration, so a temple-studded land grows faithful and tithes more.
        let resonance_bonus: f32 = buildings
            .iter()
            .filter(|b| b.settlement_id == settlement.id)
            .map(|b| b.resonance_bonus)
            .sum();
        if resonance_bonus > 0.0 {
            regions[idx].add_resonance(resonance_bonus);
        }

        // A settlement's granaries lay up grain against the lean years (GDD 6 <->
        // 5.3): every Granary keeps its region's stock a little fuller each tick, a
        // built buffer against famine beside a fertile field and a hallowed harvest,
        // so a well-stored land tips into dearth less readily and breaks it sooner.
        let harvest_bonus: f32 = buildings
            .iter()
            .filter(|b| b.settlement_id == settlement.id)
            .map(|b| b.harvest_bonus)
            .sum();
        if harvest_bonus > 0.0 {
            regions[idx].add_harvest(harvest_bonus);
        }
        let target = supporting;

        // The land feeds only so many: population swells toward a capacity set by
        // its supporting prosperity, then holds, rather than compounding forever.
        let capacity = balance.capacity_per_prosperity * supporting;
        let rate = settlement.growth_rate(region_prosperity, region_chaos, balance);
        let growth = settlement.capacity_limited_growth(rate, capacity);
        let tier_before = settlement.tier(&balance.tier_thresholds);
        settlement.population = (settlement.population * (1.0 + growth)).max(0.0);
        settlement.prosperity =
            approach(settlement.prosperity, target, balance.prosperity_drift_rate)
                .clamp(0.0, 100.0);

        // A settlement crossing a size threshold is a chronicled milestone: a
        // village swelling into a town, or a city dwindling as its people leave.
        // Growth is gradual, so at most one tier is crossed per tick.
        let tier_after = settlement.tier(&balance.tier_thresholds);
        if tier_after != tier_before {
            if let Some(name) = tier_names.get(tier_after) {
                let (line, kind) = if tier_after > tier_before {
                    (&text.settlement_ascends, EventKind::Region)
                } else {
                    (&text.settlement_declines, EventKind::Region)
                };
                chronicle.push(
                    year,
                    kind,
                    fill(
                        line,
                        &[
                            ("settlement", settlement.name.clone()),
                            ("tier", name.clone()),
                            ("region", regions[idx].name.clone()),
                        ],
                    ),
                );
            }
        }

        let contribution = settlement.region_contribution(balance);
        regions[idx].apply_deltas(contribution, 0.0, 0.0, 0.0, region_balance);
    }
}

/// Remove settlements whose population has collapsed below the abandonment floor
/// (GDD 5.3), and with them the buildings they held — a town emptied by war and
/// famine finally passes from the map rather than lingering as a ghost town.
pub fn tick_settlement_abandonment(
    settlements: &mut Vec<Settlement>,
    buildings: &mut Vec<Building>,
    balance: &SettlementBalance,
    regions: &[Region],
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    let mut abandoned_ids: Vec<String> = Vec::new();
    for settlement in settlements.iter() {
        if settlement.population < balance.abandon_population {
            abandoned_ids.push(settlement.id.clone());
            let region = regions
                .iter()
                .find(|r| r.id == settlement.region_id)
                .map(|r| r.name.clone())
                .unwrap_or_else(|| settlement.region_id.clone());
            chronicle.push(
                year,
                EventKind::Region,
                fill(
                    &text.settlement_abandoned,
                    &[("settlement", settlement.name.clone()), ("region", region)],
                ),
            );
        }
    }
    if abandoned_ids.is_empty() {
        return;
    }
    settlements.retain(|s| !abandoned_ids.contains(&s.id));
    buildings.retain(|b| !abandoned_ids.contains(&b.settlement_id));
}

/// A prosperous, populous region raises a new town over time (GDD 5.3), the
/// mirror of abandonment — so a flourishing land grows fresh settlements and a
/// frontier region born townless comes to be settled. The town starts small and
/// grows through the settlement system like any other.
#[allow(clippy::too_many_arguments)]
pub fn tick_settlement_founding(
    settlements: &mut Vec<Settlement>,
    regions: &[Region],
    seq: &mut u64,
    names: &SettlementNameBank,
    balance: &SettlementBalance,
    rng: &mut SeededRng,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    for region in regions.iter() {
        if region.prosperity < balance.found_status_min
            || region.population < balance.found_min_region_pop
        {
            continue;
        }
        let town_count = settlements
            .iter()
            .filter(|s| s.region_id == region.id)
            .count();
        if town_count >= balance.found_max_per_region {
            continue;
        }
        if !rng.chance(balance.found_chance) {
            continue;
        }

        *seq += 1;
        let name = unique_settlement_name(settlements, names, rng);
        settlements.push(Settlement::from_seed(&SettlementSeed {
            id: format!("{}-town-{}", region.id, *seq),
            name: name.clone(),
            region_id: region.id.clone(),
            population: balance.found_population,
            prosperity: region.prosperity,
        }));
        chronicle.push(
            year,
            EventKind::Region,
            fill(
                &text.settlement_founded,
                &[("settlement", name), ("region", region.name.clone())],
            ),
        );
    }
}

/// A town name from the bank (prefix + suffix), unique among existing towns.
/// Deterministic given the RNG state.
fn unique_settlement_name(
    settlements: &[Settlement],
    names: &SettlementNameBank,
    rng: &mut SeededRng,
) -> String {
    if names.prefixes.is_empty() || names.suffixes.is_empty() {
        return "New Town".to_owned();
    }
    let draw = |rng: &mut SeededRng| {
        format!(
            "{}{}",
            names.prefixes[rng.below(names.prefixes.len())],
            names.suffixes[rng.below(names.suffixes.len())],
        )
    };
    // A handful of draws almost always lands a free name (hundreds of combos);
    // if the map is somehow saturated, an ordinal guarantees uniqueness.
    for _ in 0..16 {
        let candidate = draw(rng);
        if settlements.iter().all(|s| s.name != candidate) {
            return candidate;
        }
    }
    let base = draw(rng);
    (2..)
        .map(|n| format!("{base} {n}"))
        .find(|c| settlements.iter().all(|s| &s.name != c))
        .unwrap_or(base)
}

/// Prosperous, populous settlements raise new buildings over time (GDD 6). A
/// settlement holds at most one of each building type; the chosen type is drawn
/// deterministically from the world RNG (candidates sorted for determinism, as
/// the type registry is a hash map).
#[allow(clippy::too_many_arguments)]
pub fn tick_construction(
    settlements: &[Settlement],
    regions: &[Region],
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
        if candidates.is_empty() {
            continue;
        }

        // Favour a building that fits the region's dominant culture.
        let region_culture = regions
            .iter()
            .find(|r| r.id == settlement.region_id)
            .map(|r| r.culture);
        let weight = |t: &BuildingType| {
            build_weight(t.culture, region_culture, balance.culture_affinity_weight)
        };
        let total: f32 = candidates.iter().map(|t| weight(t)).sum();
        let mut roll = rng.next_f32() * total;
        let chosen = *candidates
            .iter()
            .find(|t| {
                roll -= weight(t);
                roll <= 0.0
            })
            .unwrap_or(&candidates[candidates.len() - 1]);

        buildings.push(Building {
            id: format!("{}_{}", settlement.id, chosen.id),
            name: format!("{} {}", settlement.name, chosen.name),
            settlement_id: settlement.id.clone(),
            type_id: chosen.id.clone(),
            prosperity_bonus: chosen.prosperity_bonus,
            culture: chosen.culture,
            resonance_bonus: chosen.resonance_bonus,
            harvest_bonus: chosen.harvest_bonus,
            synergy_resource: chosen.synergy_resource,
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

/// Whether a region holds a node of the given resource kind that is still
/// producing (not run dry), so a building drawing on that trade has raw material
/// at hand (GDD 6 <-> 5.3).
fn region_produces(
    nodes: &[ResourceNode],
    region_id: &str,
    resource: crate::data::ResourceType,
) -> bool {
    nodes.iter().any(|n| {
        n.region_id == region_id
            && n.resource_type == resource
            && n.status != ResourceStatus::Depleted
    })
}

/// Selection weight for a building type given the region's dominant culture: a
/// match is boosted by `affinity`, everything else stays at the 1.0 baseline.
fn build_weight(
    building_culture: Option<Culture>,
    region_culture: Option<Culture>,
    affinity: f32,
) -> f32 {
    match (building_culture, region_culture) {
        (Some(b), Some(r)) if b == r => 1.0 + affinity,
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::world::WorldState;

    #[test]
    fn a_building_matching_its_region_culture_is_favoured() {
        let a = 2.0;
        // A martial building in a martial land outweighs the baseline...
        assert_eq!(
            build_weight(Some(Culture::Martial), Some(Culture::Martial), a),
            3.0
        );
        // ...a mismatch stays at baseline...
        assert_eq!(
            build_weight(Some(Culture::Mercantile), Some(Culture::Martial), a),
            1.0
        );
        // ...and a culture-less building never gets the boost.
        assert_eq!(build_weight(None, Some(Culture::Martial), a), 1.0);
    }

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
            &world.resource_nodes,
            &data.balance.settlement,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            &data.strings.ui.settlement_tiers,
            world.year,
        );
        for (s, was) in world.settlements.iter().zip(before) {
            assert!(s.population > was);
        }
    }

    #[test]
    fn a_temple_hallows_the_land_around_it() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let settlement = world.settlements[0].clone();
        let ridx = world
            .regions
            .iter()
            .position(|r| r.id == settlement.region_id)
            .unwrap();
        world.regions[ridx].divine_resonance = 50.0;
        let before = world.regions[ridx].divine_resonance;

        // Isolate a single temple's contribution: clear the seed buildings, then
        // stand one temple (resonance bonus) and one secular hall (none) in the
        // settlement.
        world.buildings.clear();
        let building = |id: &str, resonance: f32| Building {
            id: id.to_owned(),
            name: id.to_owned(),
            settlement_id: settlement.id.clone(),
            type_id: id.to_owned(),
            prosperity_bonus: 3.0,
            culture: None,
            resonance_bonus: resonance,
            harvest_bonus: 0.0,
            synergy_resource: None,
        };
        world.buildings.push(building("temple", 0.5));
        world.buildings.push(building("market", 0.0));

        tick_settlements(
            &mut world.settlements,
            &world.buildings,
            &mut world.regions,
            &world.resource_nodes,
            &data.balance.settlement,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            &data.strings.ui.settlement_tiers,
            world.year,
        );

        // Only the temple hallows the land, and by exactly its bonus.
        assert!(
            (world.regions[ridx].divine_resonance - (before + 0.5)).abs() < 1e-4,
            "a temple should raise its region's resonance by exactly its bonus"
        );
    }

    #[test]
    fn a_granary_stores_grain_against_the_dearth() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let settlement = world.settlements[0].clone();
        let ridx = world
            .regions
            .iter()
            .position(|r| r.id == settlement.region_id)
            .unwrap();
        // A middling granary, so there is room to store more (not clamped at 100).
        world.regions[ridx].harvest = 50.0;
        let before = world.regions[ridx].harvest;

        // Isolate a single granary's contribution: one granary (harvest bonus) and
        // one secular hall (none).
        world.buildings.clear();
        let building = |id: &str, harvest: f32| Building {
            id: id.to_owned(),
            name: id.to_owned(),
            settlement_id: settlement.id.clone(),
            type_id: id.to_owned(),
            prosperity_bonus: 3.0,
            culture: None,
            resonance_bonus: 0.0,
            harvest_bonus: harvest,
            synergy_resource: None,
        };
        world.buildings.push(building("granary", 0.5));
        world.buildings.push(building("market", 0.0));

        tick_settlements(
            &mut world.settlements,
            &world.buildings,
            &mut world.regions,
            &world.resource_nodes,
            &data.balance.settlement,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            &data.strings.ui.settlement_tiers,
            world.year,
        );

        // Only the granary stores grain, and by exactly its bonus.
        assert!(
            (world.regions[ridx].harvest - (before + 0.5)).abs() < 1e-4,
            "a granary should raise its region's stock by exactly its bonus"
        );
    }

    #[test]
    fn a_building_over_its_resource_earns_the_synergy_bonus() {
        // A Forge in a region with a producing Mine lifts its settlement more than
        // the same Forge over ore-less ground, and a depleted mine grants nothing
        // (GDD 6 <-> 5.3).
        use crate::data::{ResourceStatus, ResourceType};
        let data = GameData::load().unwrap();

        let equilibrium = |mine: Option<ResourceStatus>| {
            let mut world = WorldState::new(&data);
            world.settlements.truncate(1);
            let settlement_id = world.settlements[0].id.clone();
            let region_id = world.settlements[0].region_id.clone();
            world.settlements[0].prosperity = 0.0;
            // Hold the region steady so only the building bonus moves prosperity.
            if let Some(r) = world.regions.iter_mut().find(|r| r.id == region_id) {
                r.prosperity = 50.0;
                r.chaos = 0.0;
            }
            // One Forge, drawing on Mine ore.
            world.buildings.clear();
            world.buildings.push(Building {
                id: "forge".to_owned(),
                name: "Forge".to_owned(),
                settlement_id,
                type_id: "forge".to_owned(),
                prosperity_bonus: 5.0,
                culture: None,
                resonance_bonus: 0.0,
                harvest_bonus: 0.0,
                synergy_resource: Some(ResourceType::Mine),
            });
            // The region's only node, if any, is the given mine.
            world.resource_nodes.retain(|n| n.region_id != region_id);
            if let Some(status) = mine {
                world.resource_nodes.push(ResourceNode {
                    id: "m".to_owned(),
                    name: "The Vein".to_owned(),
                    region_id: region_id.clone(),
                    resource_type: ResourceType::Mine,
                    status,
                });
            }

            // Settle prosperity toward its supported equilibrium.
            for _ in 0..400 {
                if let Some(r) = world.regions.iter_mut().find(|r| r.id == region_id) {
                    r.prosperity = 50.0;
                }
                tick_settlements(
                    &mut world.settlements,
                    &world.buildings,
                    &mut world.regions,
                    &world.resource_nodes,
                    &data.balance.settlement,
                    &data.balance.region,
                    &mut world.chronicle,
                    &data.strings.chronicle,
                    &data.strings.ui.settlement_tiers,
                    world.year,
                );
            }
            world.settlements[0].prosperity
        };

        let over_ore = equilibrium(Some(ResourceStatus::Active));
        let barren = equilibrium(None);
        let over_dry = equilibrium(Some(ResourceStatus::Depleted));
        assert!(
            over_ore > barren,
            "a Forge over a working mine should out-produce one over none ({over_ore} vs {barren})"
        );
        assert!(
            (over_dry - barren).abs() < 1e-3,
            "a Forge over a run-dry mine earns no synergy ({over_dry} vs {barren})"
        );
    }

    #[test]
    fn population_is_bounded_by_carrying_capacity() {
        // Held at high prosperity indefinitely, a settlement swells then plateaus
        // rather than compounding without limit (GDD 5.3). Supporting prosperity
        // never exceeds 100, so population can never pass capacity_per_prosperity
        // * 100 — the growth is genuinely bounded, not merely slow.
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let ceiling = data.balance.settlement.capacity_per_prosperity * 100.0;
        for _ in 0..3000 {
            for region in &mut world.regions {
                region.prosperity = 80.0;
                region.chaos = 5.0;
            }
            tick_settlements(
                &mut world.settlements,
                &world.buildings,
                &mut world.regions,
                &world.resource_nodes,
                &data.balance.settlement,
                &data.balance.region,
                &mut world.chronicle,
                &data.strings.chronicle,
                &data.strings.ui.settlement_tiers,
                world.year,
            );
        }
        let biggest = world
            .settlements
            .iter()
            .map(|s| s.population)
            .fold(0.0_f32, f32::max);
        assert!(
            biggest < ceiling,
            "population should stay under the carrying-capacity ceiling: {biggest} vs {ceiling}"
        );
        assert!(
            biggest > 20_000.0,
            "a long-prosperous settlement should still have grown well past its seed: {biggest}"
        );
    }

    #[test]
    fn a_thriving_region_founds_a_new_town() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut balance = data.balance.settlement.clone();
        balance.found_chance = 1.0; // guaranteed this tick
        balance.found_max_per_region = 100; // don't cap in the test

        let region_id = world.regions[0].id.clone();
        world.regions[0].prosperity = 90.0;
        world.regions[0].population = 50_000.0;
        let before = world
            .settlements
            .iter()
            .filter(|s| s.region_id == region_id)
            .count();
        let mut seq = 0;

        tick_settlement_founding(
            &mut world.settlements,
            &world.regions,
            &mut seq,
            &data.settlement_names,
            &balance,
            &mut world.rng,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );

        let founded: Vec<&Settlement> = world
            .settlements
            .iter()
            .filter(|s| s.region_id == region_id && s.id.contains("-town-"))
            .collect();
        assert_eq!(founded.len(), 1, "a thriving region should found one town");
        assert_eq!(
            founded[0].population, balance.found_population,
            "a new town starts with the founding population"
        );
        let after = world
            .settlements
            .iter()
            .filter(|s| s.region_id == region_id)
            .count();
        assert_eq!(after, before + 1);
        // Every settlement name stays unique across the map.
        let mut names: Vec<&str> = world.settlements.iter().map(|s| s.name.as_str()).collect();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(total, names.len(), "no two settlements share a name");
    }

    #[test]
    fn a_collapsed_settlement_is_abandoned_with_its_buildings() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        // Bleed one settlement dry; it and its buildings should pass from the map,
        // while a healthy neighbour endures.
        let doomed = world.settlements[0].id.clone();
        let doomed_region = world.settlements[0].region_id.clone();
        world.settlements[0].population = data.balance.settlement.abandon_population - 1.0;
        let survivor = world.settlements[1].id.clone();
        let before_buildings = world.buildings.len();
        let doomed_buildings = world
            .buildings
            .iter()
            .filter(|b| b.settlement_id == doomed)
            .count();

        tick_settlement_abandonment(
            &mut world.settlements,
            &mut world.buildings,
            &data.balance.settlement,
            &world.regions,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );

        assert!(
            !world.settlements.iter().any(|s| s.id == doomed),
            "the collapsed settlement should be abandoned"
        );
        assert!(
            world.settlements.iter().any(|s| s.id == survivor),
            "a healthy settlement should endure"
        );
        assert_eq!(
            world.buildings.len(),
            before_buildings - doomed_buildings,
            "the abandoned settlement's buildings should be gone"
        );
        // The passing is chronicled with the region it emptied from.
        let region_name = world
            .regions
            .iter()
            .find(|r| r.id == doomed_region)
            .map(|r| r.name.as_str())
            .unwrap_or_default();
        assert!(
            world
                .chronicle
                .iter_newest()
                .any(|e| e.message.contains("abandoned") && e.message.contains(region_name)),
            "abandonment should be chronicled"
        );
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
                &world.resource_nodes,
                &data.balance.settlement,
                &data.balance.region,
                &mut world.chronicle,
                &data.strings.chronicle,
                &data.strings.ui.settlement_tiers,
                world.year,
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
                &world.regions,
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
