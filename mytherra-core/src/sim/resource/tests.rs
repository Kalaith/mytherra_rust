use super::*;
use crate::data::GameData;
use crate::world::WorldState;

#[test]
fn depleted_node_contributes_negatively() {
    let data = GameData::load().unwrap();
    let contribution =
        (data.balance.resource.outputs.depleted - 1.0) * data.balance.resource.region_output_scale;
    assert!(contribution < 0.0);
    let flourishing = (data.balance.resource.outputs.flourishing - 1.0)
        * data.balance.resource.region_output_scale;
    assert!(flourishing > 0.0);
}

#[test]
fn only_hazardous_nodes_poison_their_region() {
    let b = GameData::load().unwrap().balance.resource;
    assert_eq!(
        status_hazard(ResourceStatus::Corrupted, &b),
        (b.corrupted_chaos, 0.0)
    );
    assert_eq!(
        status_hazard(ResourceStatus::Unstable, &b),
        (0.0, b.unstable_danger)
    );
    assert_eq!(status_hazard(ResourceStatus::Flourishing, &b), (0.0, 0.0));
    assert_eq!(status_hazard(ResourceStatus::Active, &b), (0.0, 0.0));
}

#[test]
fn a_flourishing_manaspring_wells_up_magic_not_prosperity() {
    // A thriving manaspring raises its region's magic affinity and leaves its
    // prosperity untouched, while a farm does the reverse (GDD 5.3 <-> 5.6).
    let data = GameData::load().unwrap();
    // Freeze the status machine so the node stays put, isolating its yield.
    let mut balance = data.balance.resource.clone();
    balance.degrade_base = 0.0;
    balance.degrade_stress = 0.0;
    balance.recover_base = 0.0;
    balance.improve_base = 0.0;

    let gains = |resource_type: ResourceType| {
        let mut world = WorldState::new(&data);
        world.regions.truncate(1);
        world.regions[0].prosperity = 50.0;
        world.regions[0].magic_affinity = 50.0;
        world.regions[0].chaos = 20.0; // calm, so no degrade/hazard
        world.regions[0].danger = 20.0;
        let region_id = world.regions[0].id.clone();
        world.resource_nodes.truncate(1);
        world.resource_nodes[0].resource_type = resource_type;
        world.resource_nodes[0].region_id = region_id;
        world.resource_nodes[0].status = ResourceStatus::Flourishing;
        tick_resources(
            &mut world.resource_nodes,
            &mut world.regions,
            &mut world.rng,
            &balance,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        (
            world.regions[0].prosperity - 50.0,
            world.regions[0].magic_affinity - 50.0,
        )
    };

    let (mana_prosp, mana_magic) = gains(ResourceType::Manaspring);
    assert!(
        mana_magic > 0.0 && mana_prosp.abs() < f32::EPSILON,
        "a manaspring should feed magic, not prosperity ({mana_prosp}, {mana_magic})"
    );
    let (farm_prosp, farm_magic) = gains(ResourceType::Farmland);
    assert!(
        farm_prosp > 0.0 && farm_magic.abs() < f32::EPSILON,
        "a farm should feed prosperity, not magic ({farm_prosp}, {farm_magic})"
    );
}

#[test]
fn a_corrupted_node_bleeds_chaos_into_its_region() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    // Freeze the state machine (no degrade/recover) so the node stays
    // Corrupted this tick, isolating the hazard bleed.
    let mut balance = data.balance.resource.clone();
    balance.degrade_base = 0.0;
    balance.degrade_stress = 0.0;
    balance.recover_base = 0.0;

    world.resource_nodes.truncate(1);
    world.resource_nodes[0].status = ResourceStatus::Corrupted;
    let region_id = world.resource_nodes[0].region_id.clone();
    let ridx = world
        .regions
        .iter()
        .position(|r| r.id == region_id)
        .unwrap();
    world.regions[ridx].chaos = 30.0;
    let chaos_before = world.regions[ridx].chaos;

    tick_resources(
        &mut world.resource_nodes,
        &mut world.regions,
        &mut world.rng,
        &balance,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    assert_eq!(world.resource_nodes[0].status, ResourceStatus::Corrupted);
    assert!(
        world.regions[ridx].chaos > chaos_before,
        "a corrupted node should spread chaos into its region"
    );
}

#[test]
fn a_node_falling_into_a_dramatic_state_is_chronicled() {
    // Force a Contested node in a chaos+danger-wracked region to corrupt this
    // tick, and confirm the fall is written into the chronicle by name.
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut balance = data.balance.resource.clone();
    balance.corrupt_base = 1.0; // certain corruption from Contested
    balance.recover_base = 0.0;

    world.resource_nodes.truncate(1);
    world.resource_nodes[0].status = ResourceStatus::Contested;
    let node_name = world.resource_nodes[0].name.clone();
    let region_id = world.resource_nodes[0].region_id.clone();
    let ridx = world
        .regions
        .iter()
        .position(|r| r.id == region_id)
        .unwrap();
    world.regions[ridx].chaos = 90.0;
    world.regions[ridx].danger = 90.0;

    tick_resources(
        &mut world.resource_nodes,
        &mut world.regions,
        &mut world.rng,
        &balance,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    assert_eq!(world.resource_nodes[0].status, ResourceStatus::Corrupted);
    assert!(
        world
            .chronicle
            .iter_newest()
            .any(|e| e.message.contains(&node_name) && e.message.contains("corruption")),
        "a node falling to corruption should be chronicled by name"
    );
}

#[test]
fn a_prospering_region_discovers_a_culture_fitting_node() {
    use crate::data::Culture;
    let data = GameData::load().unwrap();
    let mut balance = data.balance.resource.clone();
    balance.discovery_chance = 1.0; // certain this tick
    balance.discovery_max_per_region = 100; // don't cap in the test

    let mut world = WorldState::new(&data);
    world.regions.truncate(1);
    world.regions[0].culture = Culture::Mystical; // favors a manaspring
    world.regions[0].prosperity = balance.discovery_min_prosperity + 5.0;
    world.regions[0].population = balance.discovery_min_population + 100.0;
    let region_id = world.regions[0].id.clone();
    // Only pre-existing nodes belong elsewhere, so any node in this region is
    // freshly discovered.
    world.resource_nodes.retain(|n| n.region_id != region_id);
    let before = world.resource_nodes.len();

    tick_resource_discovery(
        &mut world.resource_nodes,
        &world.regions,
        &mut world.resource_seq,
        &balance,
        &mut world.rng,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    assert_eq!(
        world.resource_nodes.len(),
        before + 1,
        "a node was discovered"
    );
    let node = world
        .resource_nodes
        .iter()
        .find(|n| n.region_id == region_id)
        .unwrap();
    assert_eq!(
        node.resource_type,
        ResourceType::Manaspring,
        "a mystical land should open a manaspring"
    );
    assert_eq!(
        node.status,
        ResourceStatus::Active,
        "a fresh node starts Active — potential, not instant bounty"
    );
}

#[test]
fn a_poor_or_thin_region_discovers_nothing() {
    let data = GameData::load().unwrap();
    let mut balance = data.balance.resource.clone();
    balance.discovery_chance = 1.0;

    let mut world = WorldState::new(&data);
    world.regions.truncate(1);
    // Prosperous but under-populated: the gate holds.
    world.regions[0].prosperity = balance.discovery_min_prosperity + 5.0;
    world.regions[0].population = balance.discovery_min_population - 1.0;
    let region_id = world.regions[0].id.clone();
    world.resource_nodes.retain(|n| n.region_id != region_id);
    let before = world.resource_nodes.len();

    tick_resource_discovery(
        &mut world.resource_nodes,
        &world.regions,
        &mut world.resource_seq,
        &balance,
        &mut world.rng,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
    assert_eq!(
        world.resource_nodes.len(),
        before,
        "an under-populated region should discover nothing"
    );
}

#[test]
fn simulation_stays_deterministic() {
    let data = GameData::load().unwrap();
    let run = || {
        let mut world = WorldState::new(&data);
        for _ in 0..40 {
            tick_resources(
                &mut world.resource_nodes,
                &mut world.regions,
                &mut world.rng,
                &data.balance.resource,
                &data.balance.region,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
        }
        world
            .resource_nodes
            .iter()
            .map(|n| n.status)
            .collect::<Vec<_>>()
    };
    assert_eq!(run(), run());
}
