use super::*;
use crate::data::GameData;
use crate::world::WorldState;

fn run(world: &mut WorldState, data: &GameData, legend_bar: f32) {
    tick_saints(
        &mut world.saints,
        &world.heroes,
        &mut world.regions,
        &mut world.saint_seq,
        &data.balance.saint,
        legend_bar,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
}

fn dead_hero(id: &str, role: HeroRole, region: &str, renown: f32) -> Hero {
    Hero {
        id: id.to_owned(),
        name: format!("Hero {id}"),
        role,
        region_id: region.to_owned(),
        level: 8,
        age: 80,
        is_alive: false,
        renown,
    }
}

#[test]
fn the_holy_and_the_legendary_dead_are_raised_to_sainthood() {
    let data = GameData::load().unwrap();
    let b = &data.balance.saint;
    let legend_bar = 180.0;
    let mut world = WorldState::new(&data);
    let region = world.regions[0].id.clone();

    world.heroes = vec![
        // A dead Cleric past the renown floor — canonized for holiness.
        dead_hero(
            "cleric",
            HeroRole::Cleric,
            &region,
            b.renown_threshold + 5.0,
        ),
        // A dead Warrior past the floor but short of legend — NOT canonized.
        dead_hero(
            "warrior",
            HeroRole::Warrior,
            &region,
            b.renown_threshold + 5.0,
        ),
        // A dead Warrior who reached legend — canonized for sheer greatness.
        dead_hero("legend", HeroRole::Warrior, &region, legend_bar + 10.0),
        // A living Cleric of great renown — the living are not sainted.
        {
            let mut h = dead_hero("living", HeroRole::Cleric, &region, legend_bar + 50.0);
            h.is_alive = true;
            h
        },
    ];

    // Canonization is throttled to the year's cadence (1/tick): the worthiest
    // eligible soul is raised first — here the legend, at renown 190, ahead of
    // the cleric at 105 — and the rest follow in later years.
    run(&mut world, &data, legend_bar);
    let after_one: Vec<&str> = world.saints.iter().map(|s| s.hero_id.as_str()).collect();
    assert_eq!(
        after_one,
        vec!["legend"],
        "the most renowned dead is sainted first, one per year"
    );

    // The next year raises the cleric; the merely-renowned warrior and the
    // living are never raised.
    run(&mut world, &data, legend_bar);
    let sainted: Vec<&str> = world.saints.iter().map(|s| s.hero_id.as_str()).collect();
    assert!(sainted.contains(&"cleric"), "a holy dead Cleric is sainted");
    assert!(
        sainted.contains(&"legend"),
        "a legendary dead hero is sainted"
    );
    assert!(
        !sainted.contains(&"warrior"),
        "a merely-renowned non-Cleric is not sainted"
    );
    assert!(!sainted.contains(&"living"), "the living are not sainted");

    // Canonized once only, however many ticks pass.
    run(&mut world, &data, legend_bar);
    assert_eq!(
        world
            .saints
            .iter()
            .filter(|s| s.hero_id == "cleric")
            .count(),
        1,
        "a soul is never sainted twice"
    );
}

#[test]
fn a_saint_hallows_its_land_then_fades_from_memory() {
    let data = GameData::load().unwrap();
    let b = &data.balance.saint;
    let mut world = WorldState::new(&data);
    let region = world.regions[0].id.clone();
    world.regions[0].divine_resonance = 50.0;
    world.heroes = vec![dead_hero(
        "cleric",
        HeroRole::Cleric,
        &region,
        b.renown_threshold + 5.0,
    )];

    run(&mut world, &data, 180.0);
    assert_eq!(world.saints.len(), 1);
    assert!(
        world.regions[0].divine_resonance > 50.0,
        "a fresh saint hallows its home region's faith"
    );

    // Left to the ages, the veneration fades and the saint is forgotten.
    let mut forgotten = false;
    for _ in 0..1000 {
        run(&mut world, &data, 180.0);
        if world.saints.is_empty() {
            forgotten = true;
            break;
        }
    }
    assert!(forgotten, "a saint's memory should fade in the end");
}
