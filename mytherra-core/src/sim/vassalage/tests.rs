use super::*;
use crate::data::GameData;
use crate::world::WorldState;

#[allow(clippy::too_many_arguments)]
fn run(world: &mut WorldState, data: &GameData) {
    tick_vassalages(
        &mut world.vassalages,
        &mut world.regions,
        &world.heroes,
        &world.trade_routes,
        &mut world.vassalage_seq,
        &data.balance.vassalage,
        &data.balance.conquest,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
}

/// A world of exactly two trade-linked regions: a dominant `strong` and a far
/// weaker `weak`, both at peace.
fn overlord_and_weakling(data: &GameData) -> WorldState {
    let mut world = WorldState::new(data);
    // A year on the formation cadence, so a bond can be sworn this tick.
    world.year = 0;
    world.regions.truncate(2);
    world.heroes.clear();
    let (a, b) = (world.regions[0].id.clone(), world.regions[1].id.clone());
    // A trade road between them (the precondition for a bond).
    world.trade_routes.clear();
    world.trade_routes.push(crate::world::TradeRoute {
        id: "route".to_owned(),
        name: "Road".to_owned(),
        region_a: a.clone(),
        region_b: b.clone(),
        volume: 1.0,
    });
    // region[0] overwhelmingly strong, region[1] weak — both calm (at peace).
    world.regions[0].prosperity = 90.0;
    world.regions[0].population = 200_000.0;
    world.regions[0].chaos = 15.0;
    world.regions[0].danger = 15.0;
    world.regions[0].refresh_status(&data.balance.region);
    world.regions[1].prosperity = 40.0;
    world.regions[1].population = 3_000.0;
    world.regions[1].chaos = 15.0;
    world.regions[1].danger = 15.0;
    world.regions[1].refresh_status(&data.balance.region);
    world
}

#[test]
fn the_strong_subordinate_the_weak_in_peacetime() {
    let data = GameData::load().unwrap();
    let mut world = overlord_and_weakling(&data);
    let strong = world.regions[0].id.clone();
    let weak = world.regions[1].id.clone();

    let mut sworn = false;
    for _ in 0..400 {
        run(&mut world, &data);
        if !world.vassalages.is_empty() {
            sworn = true;
            break;
        }
    }
    assert!(
        sworn,
        "a dominant region should vassalize a far weaker neighbour"
    );
    assert_eq!(world.vassalages[0].overlord_id, strong);
    assert_eq!(world.vassalages[0].vassal_id, weak);
}

#[test]
fn a_vassal_renders_tribute_to_its_overlord() {
    let data = GameData::load().unwrap();
    let mut world = overlord_and_weakling(&data);
    let strong = world.regions[0].id.clone();
    let weak = world.regions[1].id.clone();
    // Seat the bond directly and hold the stats where tribute is owed.
    world.vassalages.push(Vassalage {
        id: "v".to_owned(),
        overlord_id: strong.clone(),
        vassal_id: weak.clone(),
        age: 1,
    });
    world.regions[1].prosperity = 80.0; // well above the tribute floor
    let vassal_before = world.regions[1].prosperity;
    run(&mut world, &data);
    let vassal_idx = world.regions.iter().position(|r| r.id == weak).unwrap();
    assert!(
        world.regions[vassal_idx].prosperity < vassal_before,
        "a vassal should render tribute, losing prosperity to its overlord"
    );
}

#[test]
fn a_vassal_grown_strong_throws_off_the_yoke() {
    let data = GameData::load().unwrap();
    let mut world = overlord_and_weakling(&data);
    let strong = world.regions[0].id.clone();
    let weak = world.regions[1].id.clone();
    world.vassalages.push(Vassalage {
        id: "v".to_owned(),
        overlord_id: strong.clone(),
        vassal_id: weak.clone(),
        age: 5,
    });
    // The vassal rises to match its overlord — now it can rebel.
    world.regions[1].prosperity = 90.0;
    world.regions[1].population = 200_000.0;
    world.regions[1].danger = 15.0;
    run(&mut world, &data);
    assert!(
        world.vassalages.is_empty(),
        "a vassal as mighty as its overlord should rebel to independence"
    );
}
