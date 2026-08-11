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
