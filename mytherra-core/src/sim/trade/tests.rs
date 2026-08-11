use super::*;
use crate::data::GameData;
use crate::world::WorldState;

#[test]
fn trade_narrows_the_prosperity_gap() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    // Force a wide gap on the Iron Road (aldermoor <-> kharzul).
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
    world.regions[ai].prosperity = 90.0;
    world.regions[ki].prosperity = 30.0;
    let gap_before = (world.regions[ai].prosperity - world.regions[ki].prosperity).abs();
    tick_trade(
        &world.trade_routes,
        &mut world.regions,
        &world.heroes,
        &world.resource_nodes,
        &[],
        &data.balance.trade,
        &data.balance.region,
    );
    let gap_after = (world.regions[ai].prosperity - world.regions[ki].prosperity).abs();
    assert!(gap_after < gap_before);
}

#[test]
fn peril_on_a_route_throttles_its_trade_income() {
    let data = GameData::load().unwrap();
    // Prosperity a safe endpoint gains from the Iron Road when its partner
    // sits at the given danger. Both endpoints start equal, so the equalize
    // term is zero and only the throttled trade bonus moves prosperity.
    let gain = |partner_danger: f32| {
        let mut world = WorldState::new(&data);
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
        world.regions[ai].prosperity = 50.0;
        world.regions[ki].prosperity = 50.0;
        world.regions[ai].danger = 0.0;
        world.regions[ki].danger = partner_danger;
        let before = world.regions[ai].prosperity;
        tick_trade(
            &world.trade_routes,
            &mut world.regions,
            &world.heroes,
            &world.resource_nodes,
            &[],
            &data.balance.trade,
            &data.balance.region,
        );
        world.regions[ai].prosperity - before
    };

    assert!(
        gain(0.0) > gain(100.0),
        "a route to a war-torn partner should carry less trade than a safe one"
    );
    assert!(
        gain(100.0) > 0.0,
        "even a perilous route still carries some trade (the min_safety floor)"
    );
}

#[test]
fn a_storm_over_an_endpoint_mires_the_caravans() {
    let data = GameData::load().unwrap();
    // Prosperity a calm endpoint gains from the Iron Road, with and without a
    // foul front sitting over its partner. Both endpoints start equal and safe,
    // so only the weather changes what the road carries.
    let gain = |storm: bool| {
        let mut world = WorldState::new(&data);
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
        world.regions[ai].prosperity = 50.0;
        world.regions[ki].prosperity = 50.0;
        world.regions[ai].danger = 0.0;
        world.regions[ki].danger = 0.0;
        let weather = if storm {
            vec![WeatherEvent {
                region_id: "kharzul".to_owned(),
                pattern_id: "tempest".to_owned(),
                pattern_name: "Tempest".to_owned(),
                intensity_name: "Strong".to_owned(),
                magnitude: 2.0,
                // Net-foul: chaos and danger outweigh prosperity and magic.
                prosperity: -1.0,
                chaos: 2.0,
                danger: 2.0,
                magic: 0.0,
            }]
        } else {
            Vec::new()
        };
        let before = world.regions[ai].prosperity;
        tick_trade(
            &world.trade_routes,
            &mut world.regions,
            &world.heroes,
            &world.resource_nodes,
            &weather,
            &data.balance.trade,
            &data.balance.region,
        );
        world.regions[ai].prosperity - before
    };

    assert!(
        gain(false) > gain(true),
        "a storm over a trade partner should cut what the road carries"
    );
}

#[test]
fn trade_narrows_the_culture_gap() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
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
    world.regions[ai].cultural_influence = 80.0;
    world.regions[ki].cultural_influence = 20.0;
    let gap_before =
        (world.regions[ai].cultural_influence - world.regions[ki].cultural_influence).abs();
    tick_trade(
        &world.trade_routes,
        &mut world.regions,
        &world.heroes,
        &world.resource_nodes,
        &[],
        &data.balance.trade,
        &data.balance.region,
    );
    let gap_after =
        (world.regions[ai].cultural_influence - world.regions[ki].cultural_influence).abs();
    assert!(
        gap_after < gap_before,
        "ideas should flow along the route, narrowing the culture gap"
    );
}

#[test]
fn trade_narrows_the_magic_gap() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    // A wide arcane gap on the Iron Road (aldermoor <-> kharzul); trade should
    // spread the attunement from the steeped land toward the barren one.
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
    world.regions[ai].magic_affinity = 80.0;
    world.regions[ki].magic_affinity = 20.0;
    let gap_before = (world.regions[ai].magic_affinity - world.regions[ki].magic_affinity).abs();
    tick_trade(
        &world.trade_routes,
        &mut world.regions,
        &world.heroes,
        &world.resource_nodes,
        &[],
        &data.balance.trade,
        &data.balance.region,
    );
    let gap_after = (world.regions[ai].magic_affinity - world.regions[ki].magic_affinity).abs();
    assert!(
        gap_after < gap_before,
        "arcana should travel the road, narrowing the magic gap"
    );
}

#[test]
fn trade_narrows_the_harvest_gap() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    // A wide granary gap on the Iron Road (aldermoor <-> kharzul), roads calm;
    // grain should flow from the full stores toward the hungry land.
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
    world.regions[ai].harvest = 80.0;
    world.regions[ki].harvest = 20.0;
    world.regions[ai].danger = 10.0;
    world.regions[ki].danger = 10.0;
    let gap_before = (world.regions[ai].harvest - world.regions[ki].harvest).abs();
    tick_trade(
        &world.trade_routes,
        &mut world.regions,
        &world.heroes,
        &world.resource_nodes,
        &[],
        &data.balance.trade,
        &data.balance.region,
    );
    let gap_after = (world.regions[ai].harvest - world.regions[ki].harvest).abs();
    assert!(
        gap_after < gap_before,
        "grain should travel the road, narrowing the harvest gap"
    );
    assert!(
        world.regions[ki].harvest > 20.0,
        "the hungry endpoint should be fed by its full-storehoused partner"
    );
}

#[test]
fn war_on_a_route_severs_the_food_it_carried() {
    // The same hungry endpoint is fed far less when war makes the road
    // perilous than when it runs safe (GDD 5.2 <-> 5.3): peril throttles the
    // grain trade just as it throttles wealth, so a besieged land starves alone.
    let data = GameData::load().unwrap();
    let fed_gain = |peril: f32| {
        let mut world = WorldState::new(&data);
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
        world.regions[ai].harvest = 90.0;
        world.regions[ki].harvest = 10.0;
        world.regions[ai].danger = peril;
        world.regions[ki].danger = peril;
        tick_trade(
            &world.trade_routes,
            &mut world.regions,
            &world.heroes,
            &world.resource_nodes,
            &[],
            &data.balance.trade,
            &data.balance.region,
        );
        world.regions[ki].harvest - 10.0
    };
    assert!(
        fed_gain(0.0) > fed_gain(100.0) * 2.0,
        "a safe road feeds the hungry far more than a war-torn one"
    );
}

#[test]
fn a_merchant_hero_swells_the_wealth_a_route_carries() {
    // The same route enriches its endpoints more when a Merchant hero plies
    // it than when the land holds none (GDD 5.2 <-> 5.4). Both endpoints start
    // equal, so the equalize term is zero and only the volume-scaled bonus
    // moves prosperity.
    let data = GameData::load().unwrap();
    let gain = |role: crate::data::HeroRole| {
        let mut world = WorldState::new(&data);
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
        world.regions[ai].prosperity = 50.0;
        world.regions[ki].prosperity = 50.0;
        world.regions[ai].danger = 0.0;
        world.regions[ki].danger = 0.0;
        // A single hero of the given role living at one endpoint.
        world.heroes = vec![Hero {
            id: "h".to_owned(),
            name: "H".to_owned(),
            role,
            region_id: world.regions[ai].id.clone(),
            level: 5,
            age: 30,
            is_alive: true,
            renown: 0.0,
        }];
        let before = world.regions[ai].prosperity;
        tick_trade(
            &world.trade_routes,
            &mut world.regions,
            &world.heroes,
            &world.resource_nodes,
            &[],
            &data.balance.trade,
            &data.balance.region,
        );
        world.regions[ai].prosperity - before
    };

    assert!(
        gain(HeroRole::Merchant) > gain(HeroRole::Warrior),
        "a merchant should carry more wealth down the road than a warrior does"
    );
}

#[test]
fn producing_resources_swell_the_wealth_a_route_carries() {
    // A route from a resource-rich land carries more than one from barren
    // ground, and a run-dry node lends nothing (GDD 5.2 <-> 5.3). Endpoints
    // start equal, so the equalize term is zero and only the volume bonus moves
    // prosperity.
    let data = GameData::load().unwrap();
    let gain = |nodes: Vec<ResourceNode>| {
        let mut world = WorldState::new(&data);
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
        world.regions[ai].prosperity = 50.0;
        world.regions[ki].prosperity = 50.0;
        world.regions[ai].danger = 0.0;
        world.regions[ki].danger = 0.0;
        world.heroes.clear(); // no merchants
        world.resource_nodes = nodes;
        let before = world.regions[ai].prosperity;
        tick_trade(
            &world.trade_routes,
            &mut world.regions,
            &world.heroes,
            &world.resource_nodes,
            &[],
            &data.balance.trade,
            &data.balance.region,
        );
        world.regions[ai].prosperity - before
    };

    let node = |status: ResourceStatus| ResourceNode {
        id: "n".to_owned(),
        name: "The Vein".to_owned(),
        region_id: "aldermoor".to_owned(),
        resource_type: crate::data::ResourceType::Mine,
        status,
    };

    let with_ore = gain(vec![node(ResourceStatus::Active)]);
    let barren = gain(vec![]);
    let run_dry = gain(vec![node(ResourceStatus::Depleted)]);
    assert!(
        with_ore > barren,
        "a route from a resource-rich land should carry more ({with_ore} vs {barren})"
    );
    assert!(
        (run_dry - barren).abs() < 1e-4,
        "a depleted node lends nothing to trade"
    );
}

#[test]
fn a_prospering_isolated_region_is_drawn_into_the_network() {
    // A fifth region, prosperous but tied to no road, should be bound into the
    // caravan network — to the richest eligible partner (GDD 5.2).
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut balance = data.balance.trade.clone();
    balance.found_chance = 1.0; // certain this tick
    balance.found_max_routes_per_region = 10; // don't cap in this test

    // A new, unconnected, prosperous region — as if just born of a fracture.
    let newcomer = Region {
        id: "frontier".to_owned(),
        name: "Frontier".to_owned(),
        ..world.regions[0].clone()
    };
    world.regions.push(newcomer);
    for r in &mut world.regions {
        r.prosperity = 70.0; // every land clears the founding gate
    }
    let before = world.trade_routes.len();

    tick_trade_founding(
        &mut world.trade_routes,
        &world.regions,
        &mut world.trade_seq,
        &balance,
        &mut world.rng,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    assert!(
        world.trade_routes.len() > before,
        "a prospering isolated region should gain at least one route"
    );
    assert!(
        world
            .trade_routes
            .iter()
            .any(|r| r.region_a == "frontier" || r.region_b == "frontier"),
        "the newcomer should be tied into the network"
    );
}

#[test]
fn a_poor_region_forges_no_route() {
    // Below the prosperity gate, no road is forged however lucky the roll.
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut balance = data.balance.trade.clone();
    balance.found_chance = 1.0;
    for r in &mut world.regions {
        r.prosperity = balance.found_min_prosperity - 10.0;
    }
    let before = world.trade_routes.len();

    tick_trade_founding(
        &mut world.trade_routes,
        &world.regions,
        &mut world.trade_seq,
        &balance,
        &mut world.rng,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
    assert_eq!(
        world.trade_routes.len(),
        before,
        "a struggling realm forges no new roads"
    );
}

#[test]
fn a_forged_route_never_duplicates_an_existing_one() {
    // Run founding hard for many ticks; every route stays a unique unordered
    // pair, so no two regions are ever bound twice.
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut balance = data.balance.trade.clone();
    balance.found_chance = 1.0;
    balance.found_max_routes_per_region = 100;
    for r in &mut world.regions {
        r.prosperity = 90.0;
    }

    for _ in 0..50 {
        tick_trade_founding(
            &mut world.trade_routes,
            &world.regions,
            &mut world.trade_seq,
            &balance,
            &mut world.rng,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
    }

    let mut pairs: Vec<(String, String)> = world
        .trade_routes
        .iter()
        .map(|r| {
            let (x, y) = (r.region_a.clone(), r.region_b.clone());
            if x <= y {
                (x, y)
            } else {
                (y, x)
            }
        })
        .collect();
    let total = pairs.len();
    pairs.sort();
    pairs.dedup();
    assert_eq!(total, pairs.len(), "no two routes bind the same pair");
}
