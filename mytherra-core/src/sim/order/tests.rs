use super::*;
use crate::data::GameData;
use crate::world::WorldState;

/// Build a roster of `n` living heroes of one role, each in region `region`.
fn roster(n: usize, role: HeroRole, region: &str) -> Vec<Hero> {
    (0..n)
        .map(|i| Hero {
            id: format!("h{i}"),
            name: format!("Hero {i}"),
            role,
            region_id: region.to_owned(),
            level: 3,
            age: 30,
            is_alive: true,
            renown: 0.0,
        })
        .collect()
}

fn run(world: &mut WorldState, data: &GameData) {
    tick_orders(
        &mut world.orders,
        &mut world.regions,
        &mut world.heroes,
        &mut world.order_seq,
        &data.balance.order,
        &data.strings.orders,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
}

#[test]
fn a_calling_reaching_critical_mass_founds_its_order() {
    let data = GameData::load().unwrap();
    let b = &data.balance.order;
    let mut world = WorldState::new(&data);
    let region = world.regions[0].id.clone();

    // Just short of the threshold: no Order.
    world.heroes = roster(b.found_min_members - 1, HeroRole::Mage, &region);
    run(&mut world, &data);
    assert!(
        world.orders.is_empty(),
        "a small fellowship founds no Order"
    );

    // At the threshold: the Arcane Circle rises, exactly once.
    world.heroes = roster(b.found_min_members, HeroRole::Mage, &region);
    run(&mut world, &data);
    run(&mut world, &data);
    assert_eq!(
        world.orders.len(),
        1,
        "a calling at critical mass founds its Order, and only one"
    );
    assert_eq!(world.orders[0].role, HeroRole::Mage);
}

#[test]
fn an_orders_prestige_climbs_then_it_disbands_as_its_ranks_thin() {
    let data = GameData::load().unwrap();
    let b = &data.balance.order;
    let mut world = WorldState::new(&data);
    let region = world.regions[0].id.clone();
    world.heroes = roster(b.found_min_members + 2, HeroRole::Warrior, &region);

    for _ in 0..40 {
        run(&mut world, &data);
    }
    assert!(
        world.orders[0].prestige > 0.0,
        "a thriving Order's prestige climbs from nothing"
    );

    // Its fellowship dies away to below the dissolution floor.
    world.heroes.truncate(b.dissolve_min_members - 1);
    run(&mut world, &data);
    assert!(world.orders.is_empty(), "an Order worn too thin disbands");
}

#[test]
fn an_order_lends_cultural_weight_to_its_chapter_regions() {
    let data = GameData::load().unwrap();
    let b = &data.balance.order;
    let mut world = WorldState::new(&data);
    let chapter = world.regions[0].id.clone();
    // Members all dwell in region 0; region 1 hosts none.
    world.heroes = roster(b.found_min_members + 4, HeroRole::Merchant, &chapter);
    world.regions[0].cultural_influence = 50.0;
    world.regions[1].cultural_influence = 50.0;

    for _ in 0..30 {
        run(&mut world, &data);
    }
    assert!(
        world.regions[0].cultural_influence > 50.0,
        "a region hosting a chapter gains cultural influence"
    );
    assert_eq!(
        world.regions[1].cultural_influence, 50.0,
        "a region with no member of the calling gains nothing"
    );
}

#[test]
fn an_arcane_order_deepens_the_magic_of_its_chapter_lands() {
    let data = GameData::load().unwrap();
    let b = &data.balance.order;

    // The magic affinity a chapter region gains under an Order of the given
    // calling, everything else held level. Two regions: one hosts the
    // fellowship, the other none.
    let magic_gain = |role: HeroRole| {
        let mut world = WorldState::new(&data);
        let chapter = world.regions[0].id.clone();
        world.heroes = roster(b.found_min_members + 4, role, &chapter);
        world.regions[0].magic_affinity = 40.0;
        world.regions[1].magic_affinity = 40.0;
        for _ in 0..30 {
            run(&mut world, &data);
        }
        (
            world.regions[0].magic_affinity - 40.0,
            world.regions[1].magic_affinity - 40.0,
        )
    };

    let (arcane_here, arcane_elsewhere) = magic_gain(HeroRole::Mage);
    assert!(
        arcane_here > 0.0,
        "a region hosting an arcane chapter deepens in magic ({arcane_here})"
    );
    assert_eq!(
        arcane_elsewhere, 0.0,
        "a region with no arcane chapter gains no magic"
    );

    // A martial Order stamps culture and renown, but never the arcane.
    let (martial_here, _) = magic_gain(HeroRole::Warrior);
    assert_eq!(
        martial_here, 0.0,
        "only an arcane calling deepens its lands' magic"
    );
}

#[test]
fn a_storied_order_lends_its_members_renown() {
    let data = GameData::load().unwrap();
    let b = &data.balance.order;
    let mut world = WorldState::new(&data);
    let region = world.regions[0].id.clone();
    // A Mages' Circle and a lone Warrior who belongs to no Order.
    world.heroes = roster(b.found_min_members + 3, HeroRole::Mage, &region);
    world.heroes.push(Hero {
        id: "outsider".to_owned(),
        name: "Unaffiliated".to_owned(),
        role: HeroRole::Warrior,
        region_id: region.clone(),
        level: 3,
        age: 30,
        is_alive: true,
        renown: 0.0,
    });

    for _ in 0..40 {
        run(&mut world, &data);
    }
    let member = world.heroes.iter().find(|h| h.id == "h0").unwrap();
    let outsider = world.heroes.iter().find(|h| h.id == "outsider").unwrap();
    assert!(
        member.renown > 0.0,
        "a member of a storied Order gains renown from the fellowship"
    );
    assert_eq!(
        outsider.renown, 0.0,
        "a hero of a calling with no Order gains no such honor"
    );
}
