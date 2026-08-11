use super::*;

#[test]
fn a_veteran_in_a_thriving_land_founds_a_frontier() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let start = world.regions.len();

    // A prosperous, populous, stable home with a veteran hero.
    world.heroes[0].region_id = world.regions[0].id.clone();
    world.heroes[0].level = 20;
    world.heroes[0].is_alive = true;

    let mut founded = false;
    for _ in 0..500 {
        let home = &mut world.regions[0];
        home.prosperity = 90.0;
        home.chaos = 10.0;
        home.danger = 10.0;
        home.population = 20000.0;
        home.refresh_status(&data.balance.region);
        // Keep the founder eligible against aging/level drift.
        world.heroes[0].level = 20;
        world.heroes[0].is_alive = true;
        tick_genesis(&mut world, &data);
        if world.regions.iter().any(|r| r.id.contains("-frontier-")) {
            founded = true;
            break;
        }
    }
    assert!(founded, "a thriving land never founded a frontier");
    let frontier = world
        .regions
        .iter()
        .find(|r| r.id.contains("-frontier-"))
        .unwrap();
    let frontier_id = frontier.id.clone();
    assert!(world.regions.len() > start);
    // The frontier carries its own civilization bookkeeping and its founder.
    assert!(world
        .civilization
        .iter()
        .any(|c| c.region_id == frontier_id));
    assert!(world.heroes.iter().any(|h| h.region_id == frontier_id));
    // The colony is linked to its motherland by a trade road from birth.
    assert!(
        world.trade_routes.iter().any(|r| r.touches(&frontier_id)),
        "a founded frontier should have a trade route home"
    );
    assert!(world
        .chronicle
        .iter_newest()
        .any(|e| e.message.contains("found the frontier")));
}
#[test]
fn a_prosperity_relic_hastens_frontier_founding() {
    use crate::data::{ArtifactFocus, ArtifactSeed};
    use crate::world::Artifact;
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    world.artifacts.clear();
    let start = world.regions.len();

    // A thriving, populous home with a veteran founder.
    world.heroes[0].region_id = world.regions[0].id.clone();
    world.heroes[0].level = 20;
    world.heroes[0].is_alive = true;
    {
        let home = &mut world.regions[0];
        home.prosperity = 90.0;
        home.chaos = 10.0;
        home.danger = 10.0;
        home.population = 20000.0;
        home.refresh_status(&data.balance.region);
    }

    // A powerful Prosperity relic drives the founding chance to certainty, so
    // a single tick suffices where the base rate would need many.
    world.artifacts.push(Artifact::from_seed(&ArtifactSeed {
        id: "horn".to_owned(),
        name: "Cornucopia".to_owned(),
        focus: ArtifactFocus::Prosperity,
        power: 100,
        instability: 0.0,
        region_id: world.regions[0].id.clone(),
    }));

    tick_genesis(&mut world, &data);
    assert!(
        world.regions.len() > start,
        "a prosperity relic did not hasten founding"
    );
    assert!(world.regions.iter().any(|r| r.id.contains("-frontier-")));
}
#[test]
fn a_struggling_land_founds_nothing() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let start = world.regions.len();
    world.heroes[0].region_id = world.regions[0].id.clone();
    world.heroes[0].level = 20;

    // Merely middling — never thriving — so no frontier is founded, and calm
    // enough that no crisis triggers a fracture or conquest either.
    for _ in 0..300 {
        for region in &mut world.regions {
            region.prosperity = 55.0;
            region.chaos = 25.0;
            region.danger = 25.0;
            region.population = 20000.0;
            region.refresh_status(&data.balance.region);
        }
        world.heroes[0].level = 20;
        world.heroes[0].is_alive = true;
        tick_genesis(&mut world, &data);
    }
    assert_eq!(world.regions.len(), start, "a non-thriving land expanded");
}
