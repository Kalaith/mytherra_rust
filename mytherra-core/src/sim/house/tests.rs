use super::*;
use crate::data::{GameData, HeroRole, HeroSeed};
use crate::world::WorldState;

fn legend(id: &str, region_id: &str, renown: f32) -> Hero {
    let mut h = Hero::from_seed(&HeroSeed {
        id: id.to_owned(),
        name: id.to_owned(),
        role: HeroRole::Warrior,
        region_id: region_id.to_owned(),
        level: 30,
        age: 40,
    });
    h.renown = renown;
    h
}

#[test]
fn a_legend_founds_a_house_seated_in_their_land() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    world.houses.clear();
    let region_id = world.regions[0].id.clone();

    found_house(
        &mut world.houses,
        &mut world.house_seq,
        "brogan",
        "Brogan Aldwin",
        200.0,
        &region_id,
        "Aldermoor",
        &data.balance.house,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    assert_eq!(world.houses.len(), 1);
    let house = &world.houses[0];
    assert!(house.name.contains("Brogan Aldwin"));
    assert_eq!(house.seat_region_id, region_id);
    assert!(house.holds("brogan"));
    assert_eq!(
        house.prestige,
        200.0 * data.balance.house.found_prestige_fraction
    );

    // A second call for the same hero founds no new house.
    found_house(
        &mut world.houses,
        &mut world.house_seq,
        "brogan",
        "Brogan Aldwin",
        200.0,
        &region_id,
        "Aldermoor",
        &data.balance.house,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
    assert_eq!(world.houses.len(), 1, "a hero founds at most one house");
}

#[test]
fn prestige_tracks_the_living_line_and_a_dead_one_fades() {
    let data = GameData::load().unwrap();
    let mut balance = data.balance.house.clone();
    balance.prestige_rate = 1.0; // snap straight to the target for the test
    let mut world = WorldState::new(&data);
    let region_id = world.regions[0].id.clone();

    // A house with one famed living member.
    world.heroes = vec![legend("scion", &region_id, 150.0)];
    world.houses = vec![House {
        id: "h".to_owned(),
        name: "The House of Test".to_owned(),
        seat_region_id: region_id,
        founder_name: "Test".to_owned(),
        member_ids: vec!["scion".to_owned()],
        prestige: 10.0,
    }];

    let tick = |world: &mut WorldState| {
        tick_houses(
            &mut world.houses,
            &world.heroes,
            &world.regions,
            &balance,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        )
    };

    tick(&mut world);
    assert_eq!(
        world.houses[0].prestige, 150.0,
        "prestige tracks the living line's renown"
    );

    // The line dies out: prestige drifts to nothing and the house is forgotten.
    world.heroes[0].is_alive = false;
    for _ in 0..50 {
        if world.houses.is_empty() {
            break;
        }
        tick(&mut world);
    }
    assert!(
        world.houses.is_empty(),
        "a house with no living blood and no standing is forgotten"
    );
}

#[test]
fn a_house_whose_seat_is_lost_follows_its_blood() {
    // When a house's seat region vanishes (conquered away), the house
    // reestablishes itself where its greatest living scion dwells (GDD 5.4
    // <-> 5.2). A house with no living blood keeps its lost seat and fades.
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let refuge = world.regions[1].id.clone();

    // A scion of a house whose seat, "lost-realm", is not on the map. The
    // scion has fled to a surviving region.
    world.heroes = vec![legend("scion", &refuge, 120.0)];
    world.houses = vec![House {
        id: "h".to_owned(),
        name: "The House of Exile".to_owned(),
        seat_region_id: "lost-realm".to_owned(),
        founder_name: "Exile".to_owned(),
        member_ids: vec!["scion".to_owned()],
        prestige: 120.0,
    }];

    tick_houses(
        &mut world.houses,
        &world.heroes,
        &world.regions,
        &data.balance.house,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    assert_eq!(
        world.houses[0].seat_region_id, refuge,
        "a displaced house reseats where its blood dwells"
    );
    assert!(
        world
            .chronicle
            .iter_newest()
            .any(|e| e.message.contains("reestablishes")),
        "the reseating is chronicled"
    );
}

#[test]
fn an_heir_born_on_a_houses_seat_inherits_its_renown() {
    // A descendant born in a house's seat region joins its line and inherits a
    // share of its prestige; one born elsewhere is baseborn.
    let data = GameData::load().unwrap();
    let balance = &data.balance.house;
    let mut world = WorldState::new(&data);
    let seat = world.regions[0].id.clone();
    let elsewhere = world.regions[1].id.clone();
    world.houses = vec![House {
        id: "h".to_owned(),
        name: "The House of Test".to_owned(),
        seat_region_id: seat.clone(),
        founder_name: "Test".to_owned(),
        member_ids: vec!["founder".to_owned()],
        prestige: 100.0,
    }];

    let far = maybe_inherit(
        &mut world.houses,
        "far",
        &elsewhere,
        balance,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
    assert_eq!(far, 0.0, "a child born off the seat inherits nothing");

    let heir = maybe_inherit(
        &mut world.houses,
        "heir-1",
        &seat,
        balance,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
    assert_eq!(
        heir,
        100.0 * balance.inherit_fraction,
        "an heir born on the seat inherits a share of the prestige"
    );
    assert!(
        world.houses[0].holds("heir-1"),
        "the heir joins its house's line"
    );
}

#[test]
fn with_no_houses_an_heir_is_baseborn() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    world.houses.clear();
    let renown = maybe_inherit(
        &mut world.houses,
        "heir",
        "aldermoor",
        &data.balance.house,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
    assert_eq!(renown, 0.0, "with no houses, a newborn inherits nothing");
}
