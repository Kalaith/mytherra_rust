use super::*;
use crate::data::{GameData, HeroSeed, LandmarkSeed};
use crate::world::WorldState;

fn run(world: &mut WorldState, data: &GameData) {
    tick_lore(
        &mut world.regions,
        &world.heroes,
        &world.landmarks,
        &world.magic_paths,
        &data.balance.lore,
    );
}

#[test]
fn a_land_of_scholars_grows_learned_and_a_barren_one_does_not() {
    let data = GameData::load().unwrap();
    let lore_after = |scholars: usize| {
        let mut world = WorldState::new(&data);
        world.regions.truncate(1);
        world.landmarks.clear();
        world.magic_paths.clear();
        let region_id = world.regions[0].id.clone();
        world.regions[0].prosperity = 50.0; // neutral, so only scholars matter
        world.regions[0].lore = 20.0;
        world.heroes = (0..scholars)
            .map(|i| {
                Hero::from_seed(&HeroSeed {
                    id: format!("s{i}"),
                    name: format!("Scholar {i}"),
                    role: HeroRole::Scholar,
                    region_id: region_id.clone(),
                    level: 5,
                    age: 30,
                })
            })
            .collect();
        for _ in 0..300 {
            run(&mut world, &data);
        }
        world.regions[0].lore
    };
    assert!(
        lore_after(4) > lore_after(0),
        "a land of scholars should grow far more learned than a barren one"
    );
}

#[test]
fn a_great_library_stores_a_lands_learning() {
    let data = GameData::load().unwrap();
    let lore_after = |with_library: bool| {
        let mut world = WorldState::new(&data);
        world.regions.truncate(1);
        world.heroes.clear();
        world.magic_paths.clear();
        let region_id = world.regions[0].id.clone();
        world.regions[0].prosperity = 50.0;
        world.regions[0].lore = 20.0;
        world.landmarks.clear();
        if with_library {
            let mut l = Landmark::from_seed(&LandmarkSeed {
                id: "lib".to_owned(),
                name: "The Great Library".to_owned(),
                region_id: region_id.clone(),
                culture: crate::data::Culture::Scholarly,
                influence: 5.0,
            });
            l.stature = 40.0;
            world.landmarks.push(l);
        }
        for _ in 0..300 {
            run(&mut world, &data);
        }
        world.regions[0].lore
    };
    assert!(
        lore_after(true) > lore_after(false),
        "a land with a great library should grow more learned than one without"
    );
}

#[test]
fn lore_reliefs_scale_from_ignorance_to_mastery() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let relief = 0.5;
    world.regions[0].lore = 0.0;
    assert_eq!(toll_relief(&world.regions[0], relief), 0.0);
    world.regions[0].lore = 100.0;
    assert_eq!(toll_relief(&world.regions[0], relief), relief);
    world.regions[0].lore = 50.0;
    assert!((toll_relief(&world.regions[0], relief) - relief * 0.5).abs() < 1e-4);
}
