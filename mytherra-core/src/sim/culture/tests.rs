use super::*;
use crate::data::GameData;
use crate::world::WorldState;

#[test]
fn the_works_a_people_raise_reinforce_their_culture() {
    // A pastoral region with no other signals, but whose one settlement holds
    // several forges, should harden martial as its works speak for it (GDD 6).
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut region = world.regions[0].clone();
    region.culture = Culture::Pastoral;
    let region_id = region.id.clone();
    let mut regions = vec![region];
    // Prosperity 0 so the settlement lends no mercantile pull of its own,
    // isolating the buildings' contribution.
    let settlements = vec![Settlement {
        id: "s".to_owned(),
        name: "S".to_owned(),
        region_id: region_id.clone(),
        population: 1000.0,
        prosperity: 0.0,
    }];
    let buildings: Vec<Building> = (0..5)
        .map(|i| Building {
            id: format!("f{i}"),
            name: "Forge".to_owned(),
            settlement_id: "s".to_owned(),
            type_id: "forge".to_owned(),
            prosperity_bonus: 0.0,
            culture: Some(Culture::Martial),
            resonance_bonus: 0.0,
            harvest_bonus: 0.0,
            synergy_resource: None,
        })
        .collect();
    let thresholds = &data.balance.settlement.tier_thresholds;
    for _ in 0..3 {
        tick_culture(
            &mut regions,
            &[],
            &[],
            &[],
            &settlements,
            &buildings,
            &[],
            &[],
            &[],
            &[],
            &[],
            &data.balance.culture,
            &data.balance.region,
            thresholds,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
    }
    assert_eq!(
        regions[0].culture,
        Culture::Martial,
        "a land of forges should harden martial"
    );
}

#[test]
fn a_saints_shrine_turns_its_land_toward_the_mystical() {
    use crate::world::Saint;
    // A martial region whose only cultural signals are the shrines of its
    // venerated dead should, given devotion enough to clear the inertia
    // margin, turn mystical — its people drawn toward the holy (GDD 5.2 <-> 5.1).
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut region = world.regions[0].clone();
    region.culture = Culture::Martial;
    let region_id = region.id.clone();
    let mut regions = vec![region];
    let saint = |id: &str| Saint {
        id: id.to_owned(),
        name: "Saint Test".to_owned(),
        hero_id: id.to_owned(),
        region_id: region_id.clone(),
        veneration: 100.0,
        canonized_year: 0,
    };
    let saints = vec![saint("s1"), saint("s2")];
    let thresholds = &data.balance.settlement.tier_thresholds;
    for _ in 0..30 {
        tick_culture(
            &mut regions,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &saints,
            &[],
            &data.balance.culture,
            &data.balance.region,
            thresholds,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
    }
    assert_eq!(
        regions[0].culture,
        Culture::Mystical,
        "a land that keeps a saint's shrine should turn to the mystical"
    );
}

#[test]
fn a_great_order_stamps_its_calling_on_its_chapter() {
    use crate::world::{Hero, Order};
    // A pastoral region hosting a chapter of a storied Warriors' Order — a few
    // resident warriors and the institution behind them — should harden martial,
    // where the same handful of warriors without the Order would not (GDD 5.2
    // <-> 5.4).
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut region = world.regions[0].clone();
    region.culture = Culture::Pastoral;
    let region_id = region.id.clone();

    // A single resident warrior — a chapter of one — whose own cultural pull
    // sits below the inertia margin, so only the Order behind them can tip it.
    let heroes: Vec<Hero> = (0..1)
        .map(|i| Hero {
            id: format!("w{i}"),
            name: format!("Warrior {i}"),
            role: HeroRole::Warrior,
            region_id: region_id.clone(),
            level: 1,
            age: 30,
            is_alive: true,
            renown: 0.0,
        })
        .collect();
    let order = Order {
        id: "o".to_owned(),
        name: "the Warriors' Order".to_owned(),
        role: HeroRole::Warrior,
        prestige: 100.0,
        founded_year: 0,
    };

    let mut flips_with = |orders: &[Order]| {
        let mut regions = vec![{
            let mut r = region.clone();
            r.culture = Culture::Pastoral;
            r
        }];
        let thresholds = &data.balance.settlement.tier_thresholds;
        for _ in 0..30 {
            tick_culture(
                &mut regions,
                &heroes,
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                orders,
                &data.balance.culture,
                &data.balance.region,
                thresholds,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
        }
        regions[0].culture == Culture::Martial
    };

    assert!(
        flips_with(std::slice::from_ref(&order)),
        "a chapter of a great Order should harden its land toward its calling"
    );
    assert!(
        !flips_with(&[]),
        "the same warriors without the Order behind them should not flip the land"
    );
}

#[test]
fn culture_role_yields_a_role_of_that_culture() {
    // Each culture's archetypal role maps back to that same culture, so heirs
    // born to a land's culture reinforce it.
    for culture in Culture::ALL {
        assert_eq!(hero_culture(culture_role(culture)), culture);
    }
    assert_eq!(culture_role(Culture::Martial), HeroRole::Warrior);
    assert_eq!(culture_role(Culture::Mercantile), HeroRole::Merchant);
}

#[test]
fn every_role_maps_to_a_culture_and_merchants_are_mercantile() {
    // A merchant is the only role that feeds Mercantile culture, filling the
    // gap the settlement/trade signals otherwise carried alone.
    assert_eq!(hero_culture(HeroRole::Merchant), Culture::Mercantile);
    assert_eq!(hero_culture(HeroRole::Cleric), Culture::Mystical);
    // The mapping is total over every declared role (would not compile
    // otherwise, but this guards the ALL list too).
    for role in HeroRole::ALL {
        let _ = hero_culture(role);
    }
}

#[test]
fn a_landmark_radiates_its_character_into_its_region() {
    // Kharzul's martial cairns and gates make the land more perilous, while
    // Sylvenmar's mystical groves deepen its magic (GDD 5.2).
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    for r in &mut world.regions {
        if r.id == "kharzul" || r.id == "sylvenmar" {
            r.danger = 40.0;
            r.magic_affinity = 40.0;
        }
    }

    tick_culture(
        &mut world.regions,
        &world.heroes,
        &world.landmarks,
        &world.resource_nodes,
        &world.settlements,
        &world.buildings,
        &world.trade_routes,
        &world.myths,
        &world.houses,
        &[],
        &[],
        &data.balance.culture,
        &data.balance.region,
        &data.balance.settlement.tier_thresholds,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    let kharzul = world.regions.iter().find(|r| r.id == "kharzul").unwrap();
    let sylvenmar = world.regions.iter().find(|r| r.id == "sylvenmar").unwrap();
    assert!(
        kharzul.danger > 40.0,
        "martial landmarks should make Kharzul more perilous: {}",
        kharzul.danger
    );
    assert!(
        sylvenmar.magic_affinity > 40.0,
        "mystical landmarks should deepen Sylvenmar's magic: {}",
        sylvenmar.magic_affinity
    );
}

#[test]
fn scholarly_landmark_and_scholar_hold_aldermoor_scholarly() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    // Aldermoor seeds Scholarly, has the Grand Library + a scholar hero;
    // it should stay Scholarly after a tick.
    tick_culture(
        &mut world.regions,
        &world.heroes,
        &world.landmarks,
        &world.resource_nodes,
        &world.settlements,
        &world.buildings,
        &world.trade_routes,
        &world.myths,
        &world.houses,
        &[],
        &[],
        &data.balance.culture,
        &data.balance.region,
        &data.balance.settlement.tier_thresholds,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
    let aldermoor = world.regions.iter().find(|r| r.id == "aldermoor").unwrap();
    assert_eq!(aldermoor.culture, Culture::Scholarly);
}

#[test]
fn culture_flips_when_challenger_clears_inertia() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    // Force Kharzul (has War Cairns + warrior) to a weak culture; martial
    // score should overcome the inertia margin and flip it back.
    if let Some(k) = world.regions.iter_mut().find(|r| r.id == "kharzul") {
        k.culture = Culture::Pastoral;
    }
    tick_culture(
        &mut world.regions,
        &world.heroes,
        &world.landmarks,
        &world.resource_nodes,
        &world.settlements,
        &world.buildings,
        &world.trade_routes,
        &world.myths,
        &world.houses,
        &[],
        &[],
        &data.balance.culture,
        &data.balance.region,
        &data.balance.settlement.tier_thresholds,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
    let kharzul = world.regions.iter().find(|r| r.id == "kharzul").unwrap();
    assert_ne!(kharzul.culture, Culture::Pastoral);
}

#[test]
fn a_great_city_pulls_mercantile_where_a_village_would_not() {
    // One settlement of prosperity 80 is the region's only culture signal.
    // A village's commerce is too weak to overcome the flip inertia, but a
    // metropolis of the same wealth is a strong enough mercantile engine to
    // turn a pastoral land over to commerce (GDD 5.2 — urbanization).
    let data = GameData::load().unwrap();
    let thresholds = &data.balance.settlement.tier_thresholds;
    let run = |population: f32| -> Culture {
        let mut world = WorldState::new(&data);
        let mut region = world.regions[0].clone();
        region.culture = Culture::Pastoral;
        let region_id = region.id.clone();
        let mut regions = vec![region];
        let settlements = vec![Settlement {
            id: "c".to_owned(),
            name: "City".to_owned(),
            region_id,
            population,
            prosperity: 80.0,
        }];
        for _ in 0..5 {
            tick_culture(
                &mut regions,
                &[],
                &[],
                &[],
                &settlements,
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                &data.balance.culture,
                &data.balance.region,
                thresholds,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
        }
        regions[0].culture
    };
    assert_eq!(
        run(2_000.0),
        Culture::Pastoral,
        "a village's commerce is too weak to flip the region"
    );
    assert_eq!(
        run(40_000.0),
        Culture::Mercantile,
        "a metropolis is a strong enough engine of commerce to flip it"
    );
}

#[test]
fn hearthmoor_holds_pastoral_over_a_long_run() {
    // Hearthmoor's rangers, farmland/forest, and Harvest Shrine should keep
    // its Pastoral identity despite the Mercantile pull of its settlements
    // and the Grain Road.
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    for _ in 0..80 {
        tick_culture(
            &mut world.regions,
            &world.heroes,
            &world.landmarks,
            &world.resource_nodes,
            &world.settlements,
            &world.buildings,
            &world.trade_routes,
            &world.myths,
            &world.houses,
            &[],
            &[],
            &data.balance.culture,
            &data.balance.region,
            &data.balance.settlement.tier_thresholds,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
    }
    let hearthmoor = world.regions.iter().find(|r| r.id == "hearthmoor").unwrap();
    assert_eq!(hearthmoor.culture, Culture::Pastoral);
}

#[test]
fn a_great_houses_seat_grows_in_cultural_influence() {
    // A region that is the seat of a prestigious noble house reverts toward a
    // higher cultural-influence target than the same region with none (GDD 5.2
    // <-> 5.4).
    use crate::world::House;
    let data = GameData::load().unwrap();

    let settled_influence = |seat_prestige: Option<f32>| {
        let mut world = WorldState::new(&data);
        world.regions.truncate(1);
        let region_id = world.regions[0].id.clone();
        world.regions[0].cultural_influence = 0.0;
        world.landmarks.clear();
        world.houses.clear();
        if let Some(prestige) = seat_prestige {
            world.houses.push(House {
                id: "h".to_owned(),
                name: "The House of Test".to_owned(),
                seat_region_id: region_id.clone(),
                founder_name: "Test".to_owned(),
                member_ids: vec!["founder".to_owned()],
                prestige,
            });
        }
        for _ in 0..200 {
            tick_culture(
                &mut world.regions,
                &[],
                &world.landmarks,
                &[],
                &[],
                &[],
                &[],
                &[],
                &world.houses,
                &[],
                &[],
                &data.balance.culture,
                &data.balance.region,
                &data.balance.settlement.tier_thresholds,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
        }
        world.regions[0].cultural_influence
    };

    assert!(
        settled_influence(Some(300.0)) > settled_influence(None),
        "a region seated by a great house should grow more culturally renowned"
    );
}

#[test]
fn a_lands_living_legends_shape_its_culture() {
    // A region whose only cultural force is a body of martial legend takes up
    // a Martial character, wherever it started (GDD 5.2 <-> 5.6).
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    world.regions.truncate(1);
    let region_id = world.regions[0].id.clone();
    let region_name = world.regions[0].name.clone();
    world.regions[0].culture = Culture::Scholarly; // start off-martial

    let myths: Vec<Myth> = (0..4)
        .map(|i| Myth {
            id: format!("m{i}"),
            title: "A Tale of Valor".to_owned(),
            theme_name: "Valor".to_owned(),
            stat: crate::data::MythStat::Prosperity,
            cultural_effect: 0.0,
            stat_effect: 0.0,
            culture: Culture::Martial,
            region_id: region_id.clone(),
            region_name: region_name.clone(),
            resonance: 100.0,
            echo_cooldown: 1_000_000, // hold them from echoing; test culture only
        })
        .collect();

    // Nothing else speaks for the land — no heroes, resources, or trade.
    for _ in 0..10 {
        tick_culture(
            &mut world.regions,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &myths,
            &[],
            &[],
            &[],
            &data.balance.culture,
            &data.balance.region,
            &data.balance.settlement.tier_thresholds,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
    }

    assert_eq!(
        world.regions[0].culture,
        Culture::Martial,
        "a land remembered for valor should grow martial"
    );
}
