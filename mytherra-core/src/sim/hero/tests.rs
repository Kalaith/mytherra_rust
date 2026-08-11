use super::*;
use crate::data::{ClimateType, Culture, GameData, HeroSeed, LandmarkSeed, RegionSeed};
use crate::world::WorldState;

#[test]
fn a_legend_earns_a_commemorative_death_line() {
    let data = GameData::load().unwrap();
    let text = &data.strings.chronicle;
    let bar = *data.balance.hero.renown.thresholds.last().unwrap();
    assert_eq!(death_line(bar + 1.0, bar, text), text.hero_legend_death);
    assert_eq!(death_line(bar - 1.0, bar, text), text.hero_death);
}

#[test]
fn a_living_cleric_tends_the_faith_of_its_home_region() {
    let data = GameData::load().unwrap();
    let balance = &data.balance.hero;
    // Two regions at the neutral resonance baseline (50): one home to clerics,
    // one barren of them.
    let mut regions = vec![
        region("home", 60.0, 20.0, 40.0, 40.0),
        region("barren", 60.0, 20.0, 40.0, 40.0),
    ];

    // At home: one living cleric (counts), a warrior (wrong role), and a
    // fallen cleric (dead). Only the living cleric should tend the faith.
    let warrior = hero("fighter", HeroRole::Warrior, "home");
    let mut fallen = hero("martyr", HeroRole::Cleric, "home");
    fallen.is_alive = false;
    let cleric = hero("holy", HeroRole::Cleric, "home");

    tick_faith(&[cleric, warrior, fallen], &mut regions, &[], balance);

    assert!(
        (regions[0].divine_resonance - (50.0 + balance.cleric_resonance_per_tick)).abs() < 1e-4,
        "exactly one living cleric should raise home resonance by one step"
    );
    assert_eq!(
        regions[1].divine_resonance, 50.0,
        "a land with no clerics keeps its faith unchanged"
    );
}

#[test]
fn affliction_drives_the_people_to_prayer() {
    use crate::world::Plague;
    let data = GameData::load().unwrap();
    let balance = &data.balance.hero;
    // Three cleric-less regions at the resonance baseline: one calm, one in
    // famine, one gripped by plague. Only the afflicted turn to the gods.
    let mut regions = vec![
        region("calm", 60.0, 20.0, 40.0, 40.0),
        region("starving", 60.0, 20.0, 40.0, 40.0),
        region("plagued", 60.0, 20.0, 40.0, 40.0),
    ];
    regions[1].famine = true;
    let plagues = vec![Plague {
        id: "p".to_owned(),
        name: "The Test Fever".to_owned(),
        region_id: "plagued".to_owned(),
        severity: 1.0,
        age: 0,
    }];

    tick_faith(&[], &mut regions, &plagues, balance);

    assert_eq!(
        regions[0].divine_resonance, 50.0,
        "a calm, unafflicted land's faith holds steady"
    );
    let expected = 50.0 + balance.affliction_resonance_per_tick;
    assert!(
        (regions[1].divine_resonance - expected).abs() < 1e-4,
        "a starving land turns to prayer"
    );
    assert!(
        (regions[2].divine_resonance - expected).abs() < 1e-4,
        "a plague-ridden land turns to prayer"
    );
}

#[test]
fn resident_warriors_garrison_their_region_and_lower_its_danger() {
    let data = GameData::load().unwrap();
    let balance = &data.balance.hero;
    // Two regions at equal danger: one garrisoned, one open.
    let mut regions = vec![
        region("held", 60.0, 40.0, 40.0, 40.0),
        region("open", 60.0, 40.0, 40.0, 40.0),
    ];

    // At the held region: a living warrior (garrisons), a cleric (wrong role),
    // and a fallen warrior (dead). Only the living warrior lowers danger.
    let warrior = hero("guard", HeroRole::Warrior, "held"); // level 5
    let cleric = hero("holy", HeroRole::Cleric, "held");
    let mut fallen = hero("martyr", HeroRole::Warrior, "held");
    fallen.is_alive = false;

    tick_garrison(
        &[warrior, cleric, fallen],
        &mut regions,
        balance,
        &data.balance.region,
    );

    // Relief is exactly the living warrior's levels times the coefficient.
    let expected = 40.0 - balance.warrior_danger_relief * 5.0;
    assert!(
        (regions[0].danger - expected).abs() < 1e-4,
        "a garrisoned land should grow safer by its warriors' levels"
    );
    assert_eq!(
        regions[1].danger, 40.0,
        "an ungarrisoned land keeps its peril"
    );
}

#[test]
fn heroes_flock_to_where_legends_dwell() {
    // Two identical regions; the one home to a famed hero draws migrating
    // heroes more often than the fameless one (GDD 5.4).
    let data = GameData::load().unwrap();
    let mig = &data.balance.hero.migration;
    let regions = vec![
        region("plain", 50.0, 20.0, 20.0, 20.0),
        region("storied", 50.0, 20.0, 20.0, 20.0),
    ];
    // The storied land is home to a living legend; the plain land to unknowns.
    let fame = [0.0, 300.0];
    let mut rng = SeededRng::new(11);
    let mut to_storied = 0;
    for _ in 0..1000 {
        // The mover hails from a third region, so both are candidates.
        if let Some(dest) = pick_destination(
            &regions,
            &[],
            &[],
            &[],
            &fame,
            "elsewhere",
            HeroRole::Warrior,
            &mut rng,
            mig,
        ) {
            if dest == "storied" {
                to_storied += 1;
            }
        }
    }
    assert!(
        to_storied > 550,
        "heroes should favour the storied land ({to_storied}/1000)"
    );
}

fn region(id: &str, prosperity: f32, danger: f32, magic: f32, culture: f32) -> Region {
    let balance = GameData::load().unwrap().balance.region;
    Region::from_seed(
        &RegionSeed {
            id: id.to_owned(),
            name: id.to_owned(),
            climate: ClimateType::Temperate,
            culture: Culture::Martial,
            prosperity,
            chaos: 30.0,
            danger,
            magic_affinity: magic,
            population: 5000.0,
            cultural_influence: culture,
            divine_resonance: 50.0,
        },
        &balance,
    )
}

fn hero(id: &str, role: HeroRole, region_id: &str) -> Hero {
    Hero::from_seed(&HeroSeed {
        id: id.to_owned(),
        name: id.to_owned(),
        role,
        region_id: region_id.to_owned(),
        level: 5,
        age: 30,
    })
}

#[test]
fn migration_weights_pull_each_role_differently() {
    let data = GameData::load().unwrap();
    let mig = &data.balance.hero.migration;
    let dangerous = region("war", 25.0, 90.0, 20.0, 20.0);
    let settled = region("haven", 90.0, 10.0, 30.0, 85.0);

    // A warrior is drawn to conflict; a scholar toward settled, cultured land.
    assert!(
        attractiveness(&dangerous, &[], 0.0, HeroRole::Warrior, mig)
            > attractiveness(&settled, &[], 0.0, HeroRole::Warrior, mig)
    );
    assert!(
        attractiveness(&settled, &[], 0.0, HeroRole::Scholar, mig)
            > attractiveness(&dangerous, &[], 0.0, HeroRole::Scholar, mig)
    );
    // A mage follows magic.
    let arcane = region("spire", 50.0, 30.0, 95.0, 40.0);
    assert!(
        attractiveness(&arcane, &[], 0.0, HeroRole::Mage, mig)
            > attractiveness(&settled, &[], 0.0, HeroRole::Mage, mig)
    );
}

#[test]
fn a_cleric_makes_pilgrimage_to_hallowed_ground() {
    let data = GameData::load().unwrap();
    let mig = &data.balance.hero.migration;
    // Two lands alike but for their faith.
    let mut hallowed = region("shrine", 60.0, 20.0, 40.0, 40.0);
    let mut faithless = region("waste", 60.0, 20.0, 40.0, 40.0);
    hallowed.divine_resonance = 95.0;
    faithless.divine_resonance = 20.0;

    // A cleric is drawn to the hallowed land above the faithless one.
    assert!(
        attractiveness(&hallowed, &[], 0.0, HeroRole::Cleric, mig)
            > attractiveness(&faithless, &[], 0.0, HeroRole::Cleric, mig),
        "a cleric should make pilgrimage toward hallowed ground"
    );
    // A warrior answers no such call, so resonance does not sway them.
    assert_eq!(
        attractiveness(&hallowed, &[], 0.0, HeroRole::Warrior, mig),
        attractiveness(&faithless, &[], 0.0, HeroRole::Warrior, mig),
        "divine resonance should not move a hero who does not answer its call"
    );
}

#[test]
fn the_lure_of_a_great_city_draws_every_role() {
    let data = GameData::load().unwrap();
    let mig = &data.balance.hero.migration;
    let land = region("aldervale", 60.0, 20.0, 40.0, 60.0);
    // The same land pulls harder when it holds a great city than none, for any
    // role — heroes seek the fame and fortune of the metropolis.
    for role in [HeroRole::Warrior, HeroRole::Scholar, HeroRole::Mage] {
        assert!(
            attractiveness(&land, &[], 4.0, role, mig) > attractiveness(&land, &[], 0.0, role, mig),
            "a great city should draw heroes of every calling"
        );
    }

    // greatest_city_tier reports the flagship city's tier among the towns.
    let thresholds = &data.balance.settlement.tier_thresholds;
    let town = |id: &str, pop: f32| Settlement {
        id: id.to_owned(),
        name: "T".to_owned(),
        region_id: "aldervale".to_owned(),
        population: pop,
        prosperity: 50.0,
    };
    let towns = vec![town("a", 800.0), town("b", 40_000.0), town("c", 3_000.0)];
    assert_eq!(
        greatest_city_tier("aldervale", &towns, thresholds),
        town("b", 40_000.0).tier(thresholds) as f32,
        "the greatest city's tier is what draws heroes"
    );
    assert_eq!(
        greatest_city_tier("elsewhere", &towns, thresholds),
        0.0,
        "a region with no towns has no city lure"
    );
}

#[test]
fn wonders_of_a_kin_culture_draw_their_heroes() {
    let data = GameData::load().unwrap();
    let mig = &data.balance.hero.migration;
    let land = region("aldervale", 60.0, 20.0, 40.0, 60.0);
    let scholarly_wonder = Landmark::from_seed(&LandmarkSeed {
        id: "w".to_owned(),
        name: "The Grand Athenaeum".to_owned(),
        region_id: "aldervale".to_owned(),
        culture: Culture::Scholarly,
        influence: 2.0,
    });
    let wonders = std::slice::from_ref(&scholarly_wonder);

    // A scholar is drawn more strongly to a land bearing a scholarly wonder...
    assert!(
        attractiveness(&land, wonders, 0.0, HeroRole::Scholar, mig)
            > attractiveness(&land, &[], 0.0, HeroRole::Scholar, mig),
        "a scholarly wonder should draw scholars"
    );
    // ...but a warrior, of a different culture, feels no such pull from it.
    assert_eq!(
        attractiveness(&land, wonders, 0.0, HeroRole::Warrior, mig),
        attractiveness(&land, &[], 0.0, HeroRole::Warrior, mig),
        "a scholarly wonder is no draw to a warrior"
    );
}

#[test]
fn warriors_gather_where_scholars_flee() {
    let data = GameData::load().unwrap();
    let mut balance = data.balance.hero.clone();
    // Sample steady-state migration, not the death/aging system: let heroes
    // move often and live indefinitely so the distribution is what's tested.
    balance.move_chance = 0.5;
    balance.death.min_chance = 0.0;
    balance.death.elder_roll = 0.0;
    balance.death.danger_divisor = 1.0e9; // war would otherwise thin the warriors
    balance.life_expectancy_base = 1.0e6;
    let mut world = WorldState::new(&data);
    // Three regions so the weighted choice actually has alternatives.
    world.regions = vec![
        region("war", 30.0, 70.0, 20.0, 20.0),
        region("haven", 85.0, 10.0, 30.0, 85.0),
        region("wild", 45.0, 45.0, 40.0, 30.0),
    ];
    // Everyone starts in the neutral middle; roles should sort themselves out.
    world.heroes = (0..12)
        .map(|i| {
            let role = if i % 2 == 0 {
                HeroRole::Warrior
            } else {
                HeroRole::Scholar
            };
            hero(&format!("h{i}"), role, "wild")
        })
        .collect();

    for _ in 0..150 {
        tick_heroes(
            &mut world.heroes,
            &world.regions,
            &world.landmarks,
            &world.settlements,
            &data.balance.settlement.tier_thresholds,
            &mut world.rng,
            &balance,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
    }

    let warriors_in_war = world
        .heroes
        .iter()
        .filter(|h| h.is_alive && h.role == HeroRole::Warrior && h.region_id == "war")
        .count();
    let scholars_in_war = world
        .heroes
        .iter()
        .filter(|h| h.is_alive && h.role == HeroRole::Scholar && h.region_id == "war")
        .count();
    assert!(
        warriors_in_war > scholars_in_war,
        "warriors ({warriors_in_war}) should out-gather scholars ({scholars_in_war}) in the war region"
    );
}

#[test]
fn renown_lowers_a_heros_danger_death() {
    let data = GameData::load().unwrap();
    let world = WorldState::new(&data);
    let mut famed = world.heroes[0].clone();
    famed.renown = 200.0;
    let mut unknown = famed.clone();
    unknown.renown = 0.0;
    assert!(
        danger_death_chance(&famed, 80.0, &data.balance.hero)
            < danger_death_chance(&unknown, 80.0, &data.balance.hero),
        "a renowned hero should be harder for danger to kill"
    );
}

#[test]
fn renown_accrues_as_heroes_level() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    for _ in 0..100 {
        tick_heroes(
            &mut world.heroes,
            &world.regions,
            &world.landmarks,
            &world.settlements,
            &data.balance.settlement.tier_thresholds,
            &mut world.rng,
            &data.balance.hero,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
    }
    assert!(
        world.heroes.iter().any(|h| h.renown > 0.0),
        "some hero should have earned renown by levelling"
    );
}

#[test]
fn heroes_age_each_tick() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let before: Vec<u32> = world.heroes.iter().map(|h| h.age).collect();
    tick_heroes(
        &mut world.heroes,
        &world.regions,
        &world.landmarks,
        &world.settlements,
        &data.balance.settlement.tier_thresholds,
        &mut world.rng,
        &data.balance.hero,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
    for (hero, before_age) in world.heroes.iter().zip(before) {
        if hero.is_alive {
            assert_eq!(hero.age, before_age + 1);
        }
    }
}

#[test]
fn simulation_is_deterministic_for_a_seed() {
    let data = GameData::load().unwrap();
    let run = || {
        let mut world = WorldState::new(&data);
        for _ in 0..50 {
            tick_heroes(
                &mut world.heroes,
                &world.regions,
                &world.landmarks,
                &world.settlements,
                &data.balance.settlement.tier_thresholds,
                &mut world.rng,
                &data.balance.hero,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
        }
        world
            .heroes
            .iter()
            .map(|h| (h.level, h.age, h.is_alive, h.region_id.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(run(), run());
}
