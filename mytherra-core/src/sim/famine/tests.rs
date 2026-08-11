use super::*;
use crate::data::GameData;
use crate::world::WorldState;

fn setup() -> (WorldState, GameData) {
    let data = GameData::load().unwrap();
    let world = WorldState::new(&data);
    (world, data)
}

#[test]
fn a_calm_prosperous_land_never_starves() {
    let (mut world, data) = setup();
    let b = &data.balance.famine;
    let region = &mut world.regions[0];
    region.chaos = 20.0;
    region.prosperity = 60.0;
    region.harvest = 60.0;
    region.famine = false;
    for _ in 0..50 {
        tick_famine(
            &mut world.regions,
            &mut world.settlements,
            &[],
            &[],
            b,
            &data.balance.lore,
            &data.balance.resource.outputs,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
    }
    assert!(
        !world.regions[0].famine,
        "a calm, prosperous land should keep its granaries full"
    );
    assert!(world.regions[0].harvest > b.onset);
}

#[test]
fn a_chaotic_land_starves_then_recovers_when_order_returns() {
    let (mut world, data) = setup();
    let b = &data.balance.famine;
    let idx = 0;
    world.regions[idx].chaos = 95.0;
    world.regions[idx].prosperity = 15.0;
    world.regions[idx].harvest = 40.0;
    world.regions[idx].famine = false;

    // Under chaos and squalor the granary drains until famine takes hold.
    let mut struck = false;
    for _ in 0..200 {
        tick_famine(
            &mut world.regions,
            &mut world.settlements,
            &[],
            &[],
            b,
            &data.balance.lore,
            &data.balance.resource.outputs,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        if world.regions[idx].famine {
            struck = true;
            break;
        }
    }
    assert!(struck, "a war-torn, wretched land should eventually starve");

    // Restore order, and the harvest returns and breaks the famine.
    world.regions[idx].chaos = 15.0;
    world.regions[idx].prosperity = 60.0;
    let mut broke = false;
    for _ in 0..200 {
        tick_famine(
            &mut world.regions,
            &mut world.settlements,
            &[],
            &[],
            b,
            &data.balance.lore,
            &data.balance.resource.outputs,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        if !world.regions[idx].famine {
            broke = true;
            break;
        }
    }
    assert!(broke, "a recovered land should break its famine");
}

#[test]
fn a_famine_thins_the_towns_it_grips() {
    let (mut world, data) = setup();
    let b = &data.balance.famine;
    let idx = 0;
    let region_id = world.regions[idx].id.clone();
    world.regions[idx].famine = true;
    world.regions[idx].harvest = 5.0;
    world.regions[idx].chaos = 90.0;
    world.regions[idx].prosperity = 10.0;
    let sidx = world
        .settlements
        .iter()
        .position(|s| s.region_id == region_id)
        .expect("seed region has a settlement");
    world.settlements[sidx].population = 10_000.0;
    let before = world.settlements[sidx].population;
    tick_famine(
        &mut world.regions,
        &mut world.settlements,
        &[],
        &[],
        b,
        &data.balance.lore,
        &data.balance.resource.outputs,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
    assert!(
        world.settlements[sidx].population < before,
        "a famine should cost its towns people"
    );
}

#[test]
fn fertile_fields_fill_the_granary_where_barren_land_starves() {
    use crate::data::{ResourceStatus, ResourceType};
    use crate::world::ResourceNode;
    let (world, data) = setup();
    let b = &data.balance.famine;
    let region_id = world.regions[0].id.clone();

    // The one-tick harvest gain a chaos-strained region draws, given its
    // resource nodes; everything else about the region held fixed.
    let gain_with = |nodes: Vec<ResourceNode>| {
        let mut world = world.clone();
        world.resource_nodes = nodes;
        world.regions[0].chaos = 55.0;
        world.regions[0].prosperity = 45.0;
        world.regions[0].harvest = 50.0;
        world.regions[0].famine = false;
        tick_famine(
            &mut world.regions,
            &mut world.settlements,
            &[],
            &world.resource_nodes,
            b,
            &data.balance.lore,
            &data.balance.resource.outputs,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        world.regions[0].harvest - 50.0
    };

    let node = |resource_type: ResourceType, status: ResourceStatus| ResourceNode {
        id: "n".to_owned(),
        name: "N".to_owned(),
        region_id: region_id.clone(),
        resource_type,
        status,
    };
    let barren = gain_with(vec![]);
    let farmed = gain_with(vec![
        node(ResourceType::Farmland, ResourceStatus::Flourishing),
        node(ResourceType::Fishery, ResourceStatus::Active),
    ]);
    let spent = gain_with(vec![node(ResourceType::Farmland, ResourceStatus::Depleted)]);
    let mined = gain_with(vec![node(ResourceType::Mine, ResourceStatus::Flourishing)]);
    assert!(
        farmed > barren,
        "fertile fields should feed the granary ({farmed} vs {barren})"
    );
    assert_eq!(
        spent, barren,
        "a depleted field yields nothing to the granary"
    );
    assert_eq!(
        mined, barren,
        "only fields and fisheries feed the granary, not a mine"
    );
}

#[test]
fn a_hallowed_land_reaps_a_blessed_harvest() {
    let (world, data) = setup();
    let b = &data.balance.famine;

    // The one-tick harvest gain a strained region draws at a given faith,
    // everything else held fixed.
    let gain_at = |resonance: f32| {
        let mut world = world.clone();
        world.regions[0].chaos = 55.0;
        world.regions[0].prosperity = 45.0;
        world.regions[0].divine_resonance = resonance;
        world.regions[0].harvest = 50.0;
        world.regions[0].famine = false;
        tick_famine(
            &mut world.regions,
            &mut world.settlements,
            &[],
            &[],
            b,
            &data.balance.lore,
            &data.balance.resource.outputs,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        world.regions[0].harvest - 50.0
    };

    let hallowed = gain_at(90.0);
    let neutral = gain_at(50.0);
    let faithless = gain_at(20.0);
    assert!(
        hallowed > neutral,
        "a hallowed land's harvest is blessed ({hallowed} vs {neutral})"
    );
    assert_eq!(
        faithless, neutral,
        "a land below the blessing floor is unblessed, not cursed"
    );
}
