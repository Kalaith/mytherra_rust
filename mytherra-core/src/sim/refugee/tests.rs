use super::*;
use crate::data::GameData;
use crate::world::WorldState;

fn run(world: &mut WorldState, data: &GameData) {
    tick_refugees(
        &mut world.settlements,
        &mut world.regions,
        &world.plagues,
        &world.monsters,
        &world.trade_routes,
        &data.balance.refugee,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
}

#[test]
fn people_flee_a_perilous_land_for_a_safe_haven() {
    // A settlement in a war-torn region sheds people to the safest region's
    // town, and population is conserved across the move (GDD 5.3).
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    // Region 0 is deadly; region 1 is a haven. Everything else neutral-safe.
    for (i, r) in world.regions.iter_mut().enumerate() {
        r.danger = if i == 0 { 90.0 } else { 5.0 };
        r.prosperity = if i == 1 { 90.0 } else { 40.0 };
    }
    let perilous_id = world.regions[0].id.clone();
    let haven_id = world.regions[1].id.clone();

    let total_before: f32 = world.settlements.iter().map(|s| s.population).sum();
    let perilous_before: f32 = world
        .settlements
        .iter()
        .filter(|s| s.region_id == perilous_id)
        .map(|s| s.population)
        .sum();
    let haven_before: f32 = world
        .settlements
        .iter()
        .filter(|s| s.region_id == haven_id)
        .map(|s| s.population)
        .sum();

    run(&mut world, &data);

    let total_after: f32 = world.settlements.iter().map(|s| s.population).sum();
    let perilous_after: f32 = world
        .settlements
        .iter()
        .filter(|s| s.region_id == perilous_id)
        .map(|s| s.population)
        .sum();
    let haven_after: f32 = world
        .settlements
        .iter()
        .filter(|s| s.region_id == haven_id)
        .map(|s| s.population)
        .sum();

    assert!(
        perilous_after < perilous_before,
        "a deadly land should lose people"
    );
    assert!(haven_after > haven_before, "the haven should take them in");
    assert!(
        (total_after - total_before).abs() < 1.0,
        "refugees move, they don't vanish: {total_before} -> {total_after}"
    );
}

#[test]
fn a_land_cut_off_from_the_roads_has_nowhere_to_flee() {
    // A deadly region with no trade road to any safe haven cannot shed its
    // people — they have no way to reach safety and must endure or perish where
    // they stand (GDD 5.3 <-> 5.2).
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    for (i, r) in world.regions.iter_mut().enumerate() {
        r.danger = if i == 0 { 90.0 } else { 5.0 };
        r.prosperity = if i == 1 { 90.0 } else { 40.0 };
    }
    let perilous_id = world.regions[0].id.clone();
    // Sever every road touching the deadly region: it is now an island.
    world.trade_routes.retain(|r| !r.touches(&perilous_id));

    let before: f32 = world
        .settlements
        .iter()
        .filter(|s| s.region_id == perilous_id)
        .map(|s| s.population)
        .sum();
    run(&mut world, &data);
    let after: f32 = world
        .settlements
        .iter()
        .filter(|s| s.region_id == perilous_id)
        .map(|s| s.population)
        .sum();
    assert_eq!(
        before, after,
        "a land cut off from the roads keeps its people — they cannot flee"
    );
}

#[test]
fn a_swollen_haven_pays_the_strain_of_the_influx() {
    // Taking in refugees strains the haven region's prosperity — the brake
    // that keeps one city from swallowing every refugee forever (GDD 5.3).
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    for (i, r) in world.regions.iter_mut().enumerate() {
        r.danger = if i == 0 { 90.0 } else { 5.0 };
        r.prosperity = if i == 1 { 90.0 } else { 40.0 };
    }
    let haven_id = world.regions[1].id.clone();
    let prosperity_before = world.regions[1].prosperity;

    run(&mut world, &data);

    let haven = world.regions.iter().find(|r| r.id == haven_id).unwrap();
    assert!(
        haven.prosperity < prosperity_before,
        "taking in refugees should strain the haven's prosperity"
    );
}

#[test]
fn a_plague_drives_people_out_even_from_a_calm_land() {
    // Peril isn't only danger: a plague pushes a region over the flee
    // threshold that its danger alone wouldn't reach (GDD 5.3).
    let data = GameData::load().unwrap();
    let b = &data.balance.refugee;

    let fled = |plagued: bool| {
        let mut world = WorldState::new(&data);
        // A middling-danger region, safe enough on its own to keep its people.
        for (i, r) in world.regions.iter_mut().enumerate() {
            r.danger = if i == 0 { b.flee_threshold - 10.0 } else { 5.0 };
            r.prosperity = if i == 1 { 90.0 } else { 40.0 };
        }
        let src = world.regions[0].id.clone();
        if plagued {
            world.plagues.push(crate::world::Plague {
                id: "p".to_owned(),
                name: "The Fever".to_owned(),
                region_id: src.clone(),
                severity: 1.0,
                age: 1,
            });
        }
        let before: f32 = world
            .settlements
            .iter()
            .filter(|s| s.region_id == src)
            .map(|s| s.population)
            .sum();
        run(&mut world, &data);
        let after: f32 = world
            .settlements
            .iter()
            .filter(|s| s.region_id == src)
            .map(|s| s.population)
            .sum();
        before - after
    };

    assert!(fled(false).abs() < 1e-3, "a safe land keeps its people");
    assert!(
        fled(true) > 0.0,
        "a plague should drive people to flee a land danger alone would not"
    );
}

#[test]
fn with_nowhere_safe_no_one_flees() {
    // If every region is perilous, there is no haven and the population holds
    // where it is — better the peril you know than the road to nowhere.
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    for r in &mut world.regions {
        r.danger = 95.0;
    }
    let before: Vec<f32> = world.settlements.iter().map(|s| s.population).collect();
    run(&mut world, &data);
    let after: Vec<f32> = world.settlements.iter().map(|s| s.population).collect();
    assert_eq!(before, after, "with no haven, no one moves");
}
