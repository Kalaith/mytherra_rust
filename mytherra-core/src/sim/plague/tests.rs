use super::*;
use crate::data::GameData;
use crate::world::WorldState;

#[allow(clippy::too_many_arguments)]
fn run(world: &mut WorldState, data: &GameData, balance: &PlagueBalance) {
    tick_plague(
        &mut world.plagues,
        &mut world.regions,
        &mut world.settlements,
        &world.heroes,
        &world.trade_routes,
        &mut world.plague_seq,
        &data.plague_names,
        balance,
        &data.balance.lore,
        &data.balance.region,
        &mut world.rng,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
}

#[test]
fn squalor_and_crowding_breed_a_plague() {
    // A crowded, destitute region should eventually take a plague, while an
    // identical but prosperous one stays far healthier (GDD 5.3).
    let data = GameData::load().unwrap();
    let outbreaks = |prosperity: f32| {
        let mut world = WorldState::new(&data);
        world.regions.truncate(1);
        world.regions[0].population = data.balance.plague.outbreak_min_population + 5000.0;
        world.regions[0].prosperity = prosperity;
        let mut count = 0;
        for _ in 0..400 {
            world.plagues.clear(); // isolate outbreak odds, not persistence
            run(&mut world, &data, &data.balance.plague);
            count += world.plagues.len();
        }
        count
    };
    assert!(
        outbreaks(10.0) > outbreaks(95.0),
        "squalor should breed far more plague than plenty"
    );
}

#[test]
fn a_sparse_region_stays_healthy() {
    // Below the crowding floor, no plague takes hold however squalid.
    let data = GameData::load().unwrap();
    let mut balance = data.balance.plague.clone();
    balance.outbreak_chance = 1.0; // would fire every tick if eligible
    let mut world = WorldState::new(&data);
    world.regions.truncate(1);
    world.regions[0].population = balance.outbreak_min_population - 1.0;
    world.regions[0].prosperity = 0.0;

    run(&mut world, &data, &balance);
    assert!(
        world.plagues.is_empty(),
        "a thinly-peopled land breeds no epidemic"
    );
}

#[test]
fn a_plague_saps_its_regions_settlement_and_wealth() {
    let data = GameData::load().unwrap();
    let mut balance = data.balance.plague.clone();
    balance.outbreak_chance = 0.0; // no new outbreaks; study the one we plant
    balance.spread_chance = 0.0;
    let mut world = WorldState::new(&data);
    let region_id = world.regions[0].id.clone();
    world.regions[0].prosperity = 60.0;
    let sidx = world
        .settlements
        .iter()
        .enumerate()
        .filter(|(_, s)| s.region_id == region_id)
        .max_by(|(_, a), (_, b)| a.population.total_cmp(&b.population))
        .map(|(i, _)| i)
        .expect("region has a settlement");
    let pop_before = world.settlements[sidx].population;
    let prosperity_before = world.regions[0].prosperity;
    world.plagues.push(Plague {
        id: "p".to_owned(),
        name: "The Test Fever".to_owned(),
        region_id,
        severity: 2.0,
        age: 0,
    });

    run(&mut world, &data, &balance);

    assert!(
        world.settlements[sidx].population < pop_before,
        "a plague should sap the settlement's people"
    );
    assert!(
        world.regions[0].prosperity < prosperity_before,
        "a plague should drag down its region's prosperity"
    );
}

#[test]
fn a_plague_leaps_along_a_trade_road() {
    // A plague in an isolated region can't spread; one on the trade network
    // leaps to a connected neighbour (GDD 5.3 <-> 5.2).
    let data = GameData::load().unwrap();
    let mut balance = data.balance.plague.clone();
    balance.outbreak_chance = 0.0;
    balance.spread_chance = 1.0; // certain to leap if a road allows it
    balance.decay_base = 0.0; // keep the parent alive across the tick
    balance.decay_prosperity_coeff = 0.0;

    let mut world = WorldState::new(&data);
    // The Iron Road ties aldermoor <-> kharzul.
    world.plagues.clear();
    world.plagues.push(Plague {
        id: "p".to_owned(),
        name: "The Iron Fever".to_owned(),
        region_id: "aldermoor".to_owned(),
        severity: 2.0,
        age: 0,
    });

    run(&mut world, &data, &balance);

    assert!(
        world.plagues.iter().any(|p| p.region_id != "aldermoor"),
        "the plague should have leapt to a connected region"
    );
}

#[test]
fn a_prosperous_land_throws_off_a_plague_sooner() {
    // The same plague fades faster in a rich region than a poor one, because
    // wealth tends the sick (GDD 5.3).
    let data = GameData::load().unwrap();
    let mut balance = data.balance.plague.clone();
    balance.outbreak_chance = 0.0;
    balance.spread_chance = 0.0;

    let ticks_to_fade = |prosperity: f32| {
        let mut world = WorldState::new(&data);
        let region_id = world.regions[0].id.clone();
        world.plagues.push(Plague {
            id: "p".to_owned(),
            name: "The Test Fever".to_owned(),
            region_id,
            severity: 3.0,
            age: 0,
        });
        let mut ticks = 0;
        while !world.plagues.is_empty() && ticks < 1000 {
            // Hold prosperity fixed so only the decay coefficient differs.
            world.regions[0].prosperity = prosperity;
            run(&mut world, &data, &balance);
            ticks += 1;
        }
        ticks
    };

    assert!(
        ticks_to_fade(95.0) < ticks_to_fade(5.0),
        "a wealthy land should throw off a plague sooner than a destitute one"
    );
}

#[test]
fn clerics_tend_the_sick_and_hasten_a_plagues_end() {
    // The same plague fades sooner in a region served by Clerics than in one
    // with none, holding wealth fixed (GDD 5.3 <-> 5.4).
    let data = GameData::load().unwrap();
    let mut balance = data.balance.plague.clone();
    balance.outbreak_chance = 0.0;
    balance.spread_chance = 0.0;

    let ticks_to_fade = |cleric_count: usize| {
        let mut world = WorldState::new(&data);
        let region_id = world.regions[0].id.clone();
        world.regions[0].prosperity = 30.0;
        // Replace the roster with exactly `cleric_count` Clerics in the region.
        world.heroes = (0..cleric_count)
            .map(|i| Hero {
                id: format!("c{i}"),
                name: format!("Cleric {i}"),
                role: HeroRole::Cleric,
                region_id: region_id.clone(),
                level: 5,
                age: 30,
                is_alive: true,
                renown: 0.0,
            })
            .collect();
        world.plagues.push(Plague {
            id: "p".to_owned(),
            name: "The Test Fever".to_owned(),
            region_id,
            severity: 3.0,
            age: 0,
        });
        let mut ticks = 0;
        while !world.plagues.is_empty() && ticks < 1000 {
            world.regions[0].prosperity = 30.0; // hold wealth fixed
            run(&mut world, &data, &balance);
            ticks += 1;
        }
        ticks
    };

    assert!(
        ticks_to_fade(3) < ticks_to_fade(0),
        "a land tended by Clerics should throw off a plague sooner"
    );
}

#[test]
fn famine_leaves_a_land_ripe_for_plague() {
    // At equal squalor, a starving region is far likelier to take a plague
    // than a fed one (GDD 5.3 <-> 5.3).
    let data = GameData::load().unwrap();
    let balance = &data.balance.plague;
    let mut world = WorldState::new(&data);
    world.regions[0].prosperity = 40.0;
    world.regions[0].famine = false;
    let fed = outbreak_chance(&world.regions[0], balance);
    world.regions[0].famine = true;
    let starving = outbreak_chance(&world.regions[0], balance);
    assert!(
        starving > fed,
        "famine should raise the odds of pestilence ({starving} vs {fed})"
    );
}

#[test]
fn a_plague_kills_harder_where_the_people_starve() {
    // The same plague, in the same region, exacts a heavier toll when the
    // land is also in famine (GDD 5.3 <-> 5.3).
    let data = GameData::load().unwrap();
    let mut balance = data.balance.plague.clone();
    balance.outbreak_chance = 0.0; // no fresh outbreaks to muddy the toll
    balance.spread_chance = 0.0;

    let loss_with_famine = |famine: bool| {
        let mut world = WorldState::new(&data);
        world.regions.truncate(1);
        let region_id = world.regions[0].id.clone();
        world.regions[0].prosperity = 40.0;
        world.regions[0].famine = famine;
        let sidx = world
            .settlements
            .iter()
            .position(|s| s.region_id == region_id)
            .expect("seed region has a settlement");
        world.settlements[sidx].population = 20_000.0;
        world.plagues.push(Plague {
            id: "p".to_owned(),
            name: "The Test Fever".to_owned(),
            region_id,
            severity: 2.0,
            age: 0,
        });
        let before = world.settlements[sidx].population;
        run(&mut world, &data, &balance);
        before - world.settlements[sidx].population
    };
    assert!(
        loss_with_famine(true) > loss_with_famine(false),
        "a starving people should die of plague faster than a fed one"
    );
}
