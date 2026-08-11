use super::*;
use crate::data::GameData;
use crate::world::WorldState;

fn run(world: &mut WorldState, data: &GameData, year: u32) -> Vec<FestivalRemembered> {
    world.year = year;
    tick_festivals(
        &mut world.festivals,
        &mut world.regions,
        &mut world.heroes,
        &mut world.festival_seq,
        &data.balance.festival,
        &data.strings.festivals,
        &mut world.chronicle,
        &data.strings.chronicle,
        year,
    )
}

#[test]
fn the_foremost_flourishing_realm_holds_a_festival_on_the_cadence() {
    let data = GameData::load().unwrap();
    let b = &data.balance.festival;
    let mut world = WorldState::new(&data);
    world.regions.truncate(2);
    // Region 0 is the world's cultural heart; region 1 is prominent but less so.
    for (i, r) in world.regions.iter_mut().enumerate() {
        r.prosperity = b.min_prosperity + 10.0;
        r.chaos = b.max_chaos - 5.0;
        r.cultural_influence = b.min_culture + if i == 0 { 20.0 } else { 5.0 };
    }
    let heart = world.regions[0].id.clone();

    // Off the cadence, no festival is raised.
    run(&mut world, &data, b.interval + 1);
    assert!(
        world.festivals.is_empty(),
        "no festival between the reckonings"
    );

    // On the cadence, the foremost realm holds one.
    run(&mut world, &data, b.interval);
    assert_eq!(world.festivals.len(), 1, "the cadence raises a festival");
    assert_eq!(
        world.festivals[0].region_id, heart,
        "the most culturally prominent eligible realm hosts"
    );
}

#[test]
fn a_strife_torn_or_poor_world_holds_no_festival() {
    let data = GameData::load().unwrap();
    let b = &data.balance.festival;
    let mut world = WorldState::new(&data);
    // Prominent and rich, but wracked by chaos: no celebration.
    for r in world.regions.iter_mut() {
        r.prosperity = b.min_prosperity + 10.0;
        r.cultural_influence = b.min_culture + 10.0;
        r.chaos = b.max_chaos + 20.0;
    }
    run(&mut world, &data, b.interval);
    assert!(
        world.festivals.is_empty(),
        "a strife-torn world throws no festival however rich"
    );
}

#[test]
fn a_festival_lifts_its_host_and_crowns_its_heroes_then_passes() {
    use crate::data::{HeroRole, HeroSeed};
    let data = GameData::load().unwrap();
    let b = &data.balance.festival;
    let mut world = WorldState::new(&data);
    world.regions.truncate(1);
    let host_id = world.regions[0].id.clone();
    world.regions[0].prosperity = b.min_prosperity + 10.0;
    world.regions[0].chaos = b.max_chaos - 5.0;
    world.regions[0].cultural_influence = b.min_culture + 10.0;
    world.regions[0].divine_resonance = 40.0;
    world.heroes = vec![Hero::from_seed(&HeroSeed {
        id: "reveler".to_owned(),
        name: "Reveler".to_owned(),
        role: HeroRole::Warrior,
        region_id: host_id.clone(),
        level: 3,
        age: 30,
    })];

    let culture_before = world.regions[0].cultural_influence;
    let resonance_before = world.regions[0].divine_resonance;
    let renown_before = world.heroes[0].renown;

    // Begin the festival, then run it out its full duration; the final tick
    // remembers it for a myth.
    run(&mut world, &data, b.interval);
    assert_eq!(world.festivals.len(), 1);
    let mut remembered = Vec::new();
    for y in 1..=b.duration {
        remembered = run(&mut world, &data, b.interval + y);
    }

    assert!(
        world.festivals.is_empty(),
        "a festival passes into memory once its years are spent"
    );
    assert_eq!(
        remembered.len(),
        1,
        "the festival that passes is remembered for a myth"
    );
    assert_eq!(remembered[0].1, host_id, "remembered in its host land");
    assert!(
        world.regions[0].cultural_influence > culture_before,
        "a festival deepens its host's cultural renown"
    );
    assert!(
        world.regions[0].divine_resonance > resonance_before,
        "a festival deepens its host's faith"
    );
    assert!(
        world.heroes[0].renown > renown_before,
        "a festival crowns the heroes who dwell in its host"
    );
}
