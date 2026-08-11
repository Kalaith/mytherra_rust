use super::*;
use crate::data::{Culture, GameData};
use crate::world::WorldState;

fn run(world: &mut WorldState, data: &GameData, balance: &PactBalance) {
    tick_pacts(
        &mut world.pacts,
        &mut world.regions,
        &world.trade_routes,
        &world.wars,
        &mut world.pact_seq,
        balance,
        &data.balance.region,
        &mut world.rng,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
}

/// Make two trade-linked regions kin and peaceable so they may ally: the Iron
/// Road binds aldermoor <-> kharzul.
fn ready_allies(world: &mut WorldState) -> (usize, usize) {
    let ai = world
        .regions
        .iter()
        .position(|r| r.id == "aldermoor")
        .unwrap();
    let ki = world
        .regions
        .iter()
        .position(|r| r.id == "kharzul")
        .unwrap();
    for idx in [ai, ki] {
        world.regions[idx].culture = Culture::Martial;
        world.regions[idx].chaos = 20.0;
        world.regions[idx].danger = 20.0;
    }
    (ai, ki)
}

#[test]
fn kin_and_trading_peers_swear_an_alliance() {
    let data = GameData::load().unwrap();
    let mut balance = data.balance.pact.clone();
    balance.form_chance = 1.0;
    let mut world = WorldState::new(&data);
    world.pacts.clear();
    world.wars.clear();
    ready_allies(&mut world);

    run(&mut world, &data, &balance);

    assert!(
        world.pacts.iter().any(|p| p.binds("aldermoor", "kharzul")),
        "kin, trading, peaceful peers should ally"
    );
}

#[test]
fn the_belligerent_make_no_friends() {
    let data = GameData::load().unwrap();
    let mut balance = data.balance.pact.clone();
    balance.form_chance = 1.0;
    let mut world = WorldState::new(&data);
    world.pacts.clear();
    world.wars.clear();
    let (ai, _) = ready_allies(&mut world);
    world.regions[ai].danger = 90.0; // one side seethes

    run(&mut world, &data, &balance);
    assert!(
        world.pacts.is_empty(),
        "a belligerent region forges no alliance"
    );
}

#[test]
fn an_alliance_sheds_its_members_chaos() {
    let data = GameData::load().unwrap();
    let mut balance = data.balance.pact.clone();
    balance.form_chance = 0.0; // study the pact we plant
    let mut world = WorldState::new(&data);
    let (ai, ki) = ready_allies(&mut world);
    world.regions[ai].chaos = 40.0;
    world.regions[ki].chaos = 40.0;
    let before = world.regions[ai].chaos;
    world.pacts.push(Pact {
        id: "p".to_owned(),
        region_a: "aldermoor".to_owned(),
        region_b: "kharzul".to_owned(),
        age: 0,
    });

    run(&mut world, &data, &balance);
    assert!(
        world.regions[ai].chaos < before,
        "an alliance should shed its members' chaos"
    );
}

#[test]
fn an_alliance_lapses_when_cultures_diverge() {
    let data = GameData::load().unwrap();
    let mut balance = data.balance.pact.clone();
    balance.form_chance = 0.0;
    let mut world = WorldState::new(&data);
    let (ai, ki) = ready_allies(&mut world);
    world.pacts.push(Pact {
        id: "p".to_owned(),
        region_a: "aldermoor".to_owned(),
        region_b: "kharzul".to_owned(),
        age: 3,
    });
    // The two drift to different cultures.
    world.regions[ai].culture = Culture::Martial;
    world.regions[ki].culture = Culture::Mystical;

    run(&mut world, &data, &balance);
    assert!(
        world.pacts.is_empty(),
        "an alliance should lapse once its members are no longer of one culture"
    );
}
