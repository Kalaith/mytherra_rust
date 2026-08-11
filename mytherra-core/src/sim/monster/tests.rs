use super::*;
use crate::data::GameData;
use crate::world::WorldState;

fn run(world: &mut WorldState, data: &GameData, balance: &MonsterBalance) -> Vec<BeastSlain> {
    tick_monster(
        &mut world.monsters,
        &mut world.regions,
        &mut world.settlements,
        &mut world.heroes,
        &data.monster_types,
        &mut world.monster_seq,
        balance,
        &data.balance.region,
        &mut world.rng,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    )
}

#[test]
fn perilous_wilds_breed_more_beasts_than_safe_lands() {
    let data = GameData::load().unwrap();
    let emergences = |danger: f32| {
        let mut world = WorldState::new(&data);
        world.regions.truncate(1);
        world.regions[0].danger = danger;
        world.regions[0].magic_affinity = 0.0; // natural predators only
        let mut count = 0;
        for _ in 0..400 {
            world.monsters.clear(); // isolate emergence odds, not persistence
            run(&mut world, &data, &data.balance.monster);
            count += world.monsters.len();
        }
        count
    };
    assert!(
        emergences(90.0) > emergences(45.0),
        "the more perilous wilds should breed more beasts"
    );
}

#[test]
fn resident_rangers_ward_the_wilds_against_beasts() {
    // The same perilous region breeds fewer beasts when Rangers patrol it than
    // when it is left unwarded (GDD 5.2 <-> 5.4).
    use crate::data::{HeroRole, HeroSeed};
    let data = GameData::load().unwrap();
    let emergences = |rangers: usize| {
        let mut world = WorldState::new(&data);
        world.regions.truncate(1);
        world.regions[0].danger = 95.0;
        world.regions[0].magic_affinity = 0.0;
        let region_id = world.regions[0].id.clone();
        world.heroes.retain(|h| h.region_id != region_id);
        for i in 0..rangers {
            world.heroes.push(Hero::from_seed(&HeroSeed {
                id: format!("r{i}"),
                name: format!("Ranger {i}"),
                role: HeroRole::Ranger,
                region_id: region_id.clone(),
                level: 20,
                age: 30,
            }));
        }
        let mut count = 0;
        for _ in 0..600 {
            world.monsters.clear(); // isolate emergence odds
            run(&mut world, &data, &data.balance.monster);
            count += world.monsters.len();
        }
        count
    };
    assert!(
        emergences(0) > emergences(4),
        "a ranger-warded land should breed fewer beasts than an unwarded one"
    );
}

#[test]
fn hallowed_ground_wards_the_wilds_against_beasts() {
    // The same perilous region breeds fewer beasts when its faith runs deep
    // than when it is spiritually barren (GDD 5.2 <-> 5.1). No rangers, so only
    // the sacred ground stands between the land and the wild.
    let data = GameData::load().unwrap();
    let emergences = |resonance: f32| {
        let mut world = WorldState::new(&data);
        world.regions.truncate(1);
        world.regions[0].danger = 95.0;
        world.regions[0].magic_affinity = 0.0;
        world.regions[0].divine_resonance = resonance;
        let region_id = world.regions[0].id.clone();
        world.heroes.retain(|h| h.region_id != region_id);
        let mut count = 0;
        for _ in 0..2000 {
            world.monsters.clear(); // isolate emergence odds
            world.regions[0].divine_resonance = resonance; // hold faith fixed
            run(&mut world, &data, &data.balance.monster);
            count += world.monsters.len();
        }
        count
    };
    assert!(
        emergences(20.0) > emergences(100.0),
        "a hallowed land should breed fewer beasts than a faithless one"
    );
}

#[test]
fn a_calm_land_breeds_no_beasts() {
    // Below the danger floor, no beast emerges however unlucky the roll.
    let data = GameData::load().unwrap();
    let mut balance = data.balance.monster.clone();
    balance.emergence_chance = 1.0; // would fire every tick if eligible
    let mut world = WorldState::new(&data);
    world.regions.truncate(1);
    world.regions[0].danger = balance.emergence_min_danger - 1.0;

    run(&mut world, &data, &balance);
    assert!(
        world.monsters.is_empty(),
        "a settled, peaceful land breeds no monsters"
    );
}

#[test]
fn an_arcane_land_breeds_arcane_beasts() {
    // A magic-steeped region draws only arcane horrors from the bestiary.
    let data = GameData::load().unwrap();
    let mut balance = data.balance.monster.clone();
    balance.emergence_chance = 1.0;
    let mut world = WorldState::new(&data);
    world.regions.truncate(1);
    world.regions[0].danger = balance.emergence_min_danger + 10.0;
    world.regions[0].magic_affinity = balance.arcane_magic_threshold + 10.0;
    world.monsters.clear();

    run(&mut world, &data, &balance);
    assert_eq!(world.monsters.len(), 1, "a beast should emerge");
    let type_id = &world.monsters[0].type_id;
    let ty = data
        .monster_types
        .iter()
        .find(|t| &t.id == type_id)
        .unwrap();
    assert!(
        ty.arcane,
        "a magic-steeped land should breed an arcane beast"
    );
}

#[test]
fn a_beast_menaces_its_region_and_raids_its_town() {
    let data = GameData::load().unwrap();
    let mut balance = data.balance.monster.clone();
    balance.emergence_chance = 0.0; // study the beast we plant
    let mut world = WorldState::new(&data);
    let region_id = world.regions[0].id.clone();
    world.regions[0].danger = 30.0;
    // Strip any resident hunters so the beast rages unopposed.
    world.heroes.retain(|h| h.region_id != region_id);
    let sidx = world
        .settlements
        .iter()
        .enumerate()
        .filter(|(_, s)| s.region_id == region_id)
        .max_by(|(_, a), (_, b)| a.population.total_cmp(&b.population))
        .map(|(i, _)| i)
        .expect("region has a settlement");
    let pop_before = world.settlements[sidx].population;
    let danger_before = world.regions[0].danger;
    world.monsters.push(Monster {
        id: "m".to_owned(),
        name: "The Test Beast".to_owned(),
        type_id: "hill_troll".to_owned(),
        region_id,
        ferocity: 2.0,
        age: 0,
        apex: false,
    });

    run(&mut world, &data, &balance);

    assert!(
        world.regions[0].danger > danger_before,
        "a beast should make its region more perilous"
    );
    assert!(
        world.settlements[sidx].population < pop_before,
        "a beast should raid the region's largest settlement"
    );
}

#[test]
fn resident_hunters_slay_a_beast_and_the_mightiest_earns_the_renown() {
    let data = GameData::load().unwrap();
    let mut balance = data.balance.monster.clone();
    balance.emergence_chance = 0.0;
    balance.slay_per_might = 10.0; // fell it in a single tick
    let mut world = WorldState::new(&data);
    let region_id = world.regions[0].id.clone();

    // Two hunters: a mighty warrior (claims the kill) and a lesser ranger.
    world.heroes.retain(|h| h.region_id != region_id);
    use crate::data::{HeroRole, HeroSeed};
    let mut champion = Hero::from_seed(&HeroSeed {
        id: "champ".to_owned(),
        name: "Bramwell the Bold".to_owned(),
        role: HeroRole::Warrior,
        region_id: region_id.clone(),
        level: 20,
        age: 30,
    });
    champion.renown = 0.0;
    let ranger = Hero::from_seed(&HeroSeed {
        id: "ranger".to_owned(),
        name: "A Lesser Scout".to_owned(),
        role: HeroRole::Ranger,
        region_id: region_id.clone(),
        level: 3,
        age: 30,
    });
    world.heroes.push(champion);
    world.heroes.push(ranger);
    world.monsters.push(Monster {
        id: "m".to_owned(),
        name: "The Doomed Beast".to_owned(),
        type_id: "dire_pack".to_owned(),
        region_id,
        ferocity: 1.5,
        age: 0,
        apex: false,
    });

    let felled = run(&mut world, &data, &balance);

    assert!(world.monsters.is_empty(), "the beast should be slain");
    let champ = world.heroes.iter().find(|h| h.id == "champ").unwrap();
    let ranger = world.heroes.iter().find(|h| h.id == "ranger").unwrap();
    assert_eq!(
        champ.renown, balance.slay_renown,
        "the mightiest hunter should earn the renown of the kill"
    );
    assert_eq!(ranger.renown, 0.0, "the lesser hunter earns none");
    // The kill is reported so the caller can commemorate it in myth.
    assert_eq!(
        felled,
        vec![(
            "Bramwell the Bold".to_owned(),
            "The Doomed Beast".to_owned(),
            world.regions[0].id.clone()
        )],
        "the felled beast and its slayer are reported"
    );
}

#[test]
fn an_arcane_horror_resists_steel_but_falls_to_a_mage() {
    // A Warrior wears an arcane beast down only weakly; a Mage of the same
    // level answers it in kind and cuts far deeper (GDD 5.2 <-> 5.4).
    use crate::data::{HeroRole, HeroSeed};
    let data = GameData::load().unwrap();
    let balance = &data.balance.monster;

    // Ferocity lost in one tick to a single level-10 hunter of the given role,
    // set against an arcane Shadow Wyrm.
    let bite = |role: HeroRole| {
        let mut world = WorldState::new(&data);
        let mut b = balance.clone();
        b.emergence_chance = 0.0;
        let region_id = world.regions[0].id.clone();
        world.heroes.retain(|h| h.region_id != region_id);
        world.heroes.push(Hero::from_seed(&HeroSeed {
            id: "h".to_owned(),
            name: "H".to_owned(),
            role,
            region_id: region_id.clone(),
            level: 10,
            age: 30,
        }));
        world.monsters.push(Monster {
            id: "m".to_owned(),
            name: "The Wyrm".to_owned(),
            type_id: "shadow_wyrm".to_owned(), // arcane
            region_id,
            ferocity: 5.0,
            age: 0,
            apex: false,
        });
        run(&mut world, &data, &b);
        5.0 - world.monsters.first().map(|m| m.ferocity).unwrap_or(0.0)
    };

    let warrior_bite = bite(HeroRole::Warrior);
    let mage_bite = bite(HeroRole::Mage);
    assert!(
        warrior_bite > 0.0,
        "steel should still bite an arcane beast, if weakly"
    );
    assert!(
        mage_bite > warrior_bite,
        "a Mage should cut deeper into an arcane horror than a Warrior ({mage_bite} vs {warrior_bite})"
    );
}

#[test]
fn a_mage_is_no_help_against_a_natural_predator() {
    // Against a mundane beast a lone Mage lends nothing, so the pack grows
    // unchecked as if unopposed (GDD 5.2).
    use crate::data::{HeroRole, HeroSeed};
    let data = GameData::load().unwrap();
    let mut balance = data.balance.monster.clone();
    balance.emergence_chance = 0.0;
    let mut world = WorldState::new(&data);
    let region_id = world.regions[0].id.clone();
    world.heroes.retain(|h| h.region_id != region_id);
    world.heroes.push(Hero::from_seed(&HeroSeed {
        id: "mage".to_owned(),
        name: "A Mage".to_owned(),
        role: HeroRole::Mage,
        region_id: region_id.clone(),
        level: 20,
        age: 30,
    }));
    world.monsters.push(Monster {
        id: "m".to_owned(),
        name: "The Pack".to_owned(),
        type_id: "dire_pack".to_owned(), // natural
        region_id,
        ferocity: 1.0,
        age: 0,
        apex: false,
    });

    run(&mut world, &data, &balance);
    assert!(
        world.monsters[0].ferocity > 1.0,
        "a mage can't hunt a natural pack, so it grows unopposed"
    );
}

#[test]
fn an_unopposed_beast_grows_fiercer() {
    let data = GameData::load().unwrap();
    let mut balance = data.balance.monster.clone();
    balance.emergence_chance = 0.0;
    let mut world = WorldState::new(&data);
    let region_id = world.regions[0].id.clone();
    // No hunters anywhere in the region.
    world.heroes.retain(|h| h.region_id != region_id);
    world.monsters.push(Monster {
        id: "m".to_owned(),
        name: "The Growing Terror".to_owned(),
        type_id: "dire_pack".to_owned(),
        region_id,
        ferocity: 1.0,
        age: 0,
        apex: false,
    });

    run(&mut world, &data, &balance);
    assert!(
        world.monsters[0].ferocity > 1.0,
        "an unopposed beast should grow fiercer"
    );
}

#[test]
fn an_unopposed_beast_ascends_into_a_named_legendary_terror() {
    let data = GameData::load().unwrap();
    let mut balance = data.balance.monster.clone();
    balance.emergence_chance = 0.0;
    let mut world = WorldState::new(&data);
    let region_id = world.regions[0].id.clone();
    // No hunters, and a beast already at the brink of the apex threshold.
    world.heroes.retain(|h| h.region_id != region_id);
    world.monsters.push(Monster {
        id: "monster-7".to_owned(),
        name: "The Dire Pack of Aldermoor".to_owned(),
        type_id: "dire_pack".to_owned(),
        region_id,
        ferocity: balance.apex_ferocity - balance.ferocity_growth * 0.5,
        age: 80,
        apex: false,
    });

    run(&mut world, &data, &balance);
    let beast = &world.monsters[0];
    assert!(beast.apex, "a beast past the threshold should ascend");
    assert!(
        beast.name.contains("The Dire Pack of Aldermoor")
            && beast.name.len() > "The Dire Pack of Aldermoor".len(),
        "an ascended beast takes a legendary epithet: {}",
        beast.name
    );
}

#[test]
fn felling_a_legendary_terror_makes_a_legend() {
    let data = GameData::load().unwrap();
    let mut balance = data.balance.monster.clone();
    balance.emergence_chance = 0.0;

    // The renown a lone hunter earns for the same kill, apex versus ordinary.
    let renown_for = |apex: bool| {
        let mut world = WorldState::new(&data);
        let region_id = world.regions[0].id.clone();
        // Exactly one high-level Warrior in the region to claim the kill.
        world.heroes.retain(|h| h.region_id != region_id);
        world.heroes.push(Hero {
            id: "hunter".to_owned(),
            name: "The Hunter".to_owned(),
            role: HeroRole::Warrior,
            region_id: region_id.clone(),
            level: 9,
            age: 30,
            is_alive: true,
            renown: 0.0,
        });
        // A beast already worn to the brink of death, so this tick fells it.
        world.monsters.push(Monster {
            id: "m".to_owned(),
            name: "The Doomed Beast".to_owned(),
            type_id: "dire_pack".to_owned(),
            region_id,
            ferocity: balance.min_ferocity + 0.001,
            age: 10,
            apex,
        });
        run(&mut world, &data, &balance);
        world
            .heroes
            .iter()
            .find(|h| h.id == "hunter")
            .unwrap()
            .renown
    };
    assert!(
        renown_for(true) > renown_for(false),
        "slaying an ascended terror should be worth far more renown than an ordinary kill"
    );
}
