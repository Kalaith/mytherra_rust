//! Integration tests for region genesis, driving the public `tick_genesis`
//! against a real `WorldState`. Kept in a sibling file so `genesis.rs` stays a
//! lean orchestrator (RustGames 600-line soft limit).

use super::*;
use crate::data::Culture;
use crate::world::WorldState;

/// Drive one region deep into turmoil and plant a capable hero there.
fn primed_world(data: &GameData) -> WorldState {
    let mut world = WorldState::new(data);
    let region = &mut world.regions[0];
    region.chaos = 95.0;
    region.danger = 95.0;
    region.prosperity = 10.0;
    region.refresh_status(&data.balance.region);
    let region_id = region.id.clone();
    world.heroes[0].region_id = region_id;
    world.heroes[0].level = 20;
    world.heroes[0].is_alive = true;
    world
}

#[test]
fn sustained_turmoil_fractures_a_region() {
    let data = GameData::load().unwrap();
    let mut world = primed_world(&data);
    let start = world.regions.len();

    // Keep the region turbulent; strife should cross the threshold and split.
    // The planted level-20 hero also shields it from conquest, so fracture —
    // not annexation — is the outcome.
    let mut fractured = false;
    for _ in 0..200 {
        world.regions[0].chaos = 95.0;
        world.regions[0].danger = 95.0;
        world.regions[0].refresh_status(&data.balance.region);
        tick_genesis(&mut world, &data);
        if world.regions.iter().any(|r| r.id.contains("-rift-")) {
            fractured = true;
            break;
        }
    }
    assert!(fractured, "a region under sustained strife never fractured");
    let child = world
        .regions
        .iter()
        .find(|r| r.id.contains("-rift-"))
        .unwrap();
    assert_eq!(child.culture, Culture::Martial);
    let child_id = child.id.clone();
    assert!(world.regions.len() > start);
    assert!(world.civilization.iter().any(|c| c.region_id == child_id));
    assert!(world.heroes.iter().any(|h| h.region_id == child_id));
    // The breakaway is wired into the trade network with a road home, not born
    // marooned — so it shares in trade and can later be reconquered (GDD 5.2).
    assert!(
        world.trade_routes.iter().any(|r| r.touches(&child_id)),
        "a new region should be linked into the trade network"
    );
    // The secession fed the world's momentum (drives Collapse-era pressure).
    assert!(world.secession_momentum > 0.0);
}

#[test]
fn a_breakaway_carries_off_its_share_of_resource_nodes() {
    // A fracture should divide the parent's resource nodes, not only its towns,
    // so a breakaway is born a full economic citizen (GDD 5.2). With certain
    // defection the split is deterministic: every node on the seceding land goes.
    let mut data = GameData::load().unwrap();
    data.balance.genesis.node_defect_chance = 1.0;
    let mut world = primed_world(&data);
    let parent_id = world.regions[0].id.clone();

    // Gather every node onto the fracturing region so there's a clear pool to
    // divide, and confirm the seed world actually has nodes to move.
    for node in &mut world.resource_nodes {
        node.region_id = parent_id.clone();
    }
    assert!(
        !world.resource_nodes.is_empty(),
        "seed world should have resource nodes"
    );

    let mut child_id = None;
    for _ in 0..200 {
        world.regions[0].chaos = 95.0;
        world.regions[0].danger = 95.0;
        world.regions[0].refresh_status(&data.balance.region);
        tick_genesis(&mut world, &data);
        if let Some(r) = world.regions.iter().find(|r| r.id.contains("-rift-")) {
            child_id = Some(r.id.clone());
            break;
        }
    }
    let child_id = child_id.expect("the region never fractured");
    assert!(
        world.resource_nodes.iter().any(|n| n.region_id == child_id),
        "a breakaway should carry off resource nodes from its parent"
    );
    assert!(
        !world
            .resource_nodes
            .iter()
            .any(|n| n.region_id == parent_id),
        "under certain defection every node on the seceding land goes with it"
    );
}

#[test]
fn a_knowledge_relic_quells_secession() {
    use crate::data::{ArtifactFocus, ArtifactSeed};
    use crate::world::Artifact;
    let data = GameData::load().unwrap();
    // Same turmoil that fractures a region in `sustained_turmoil...`, but a
    // Knowledge relic now bleeds strife faster than the crisis builds it.
    let mut world = primed_world(&data);
    // region[0] stays at index 0 (nothing before it is ever removed), and it
    // is shielded from conquest by its lvl-20 hero, so a fracture is the only
    // way it could reshape.
    let base_id = world.regions[0].id.clone();
    world.artifacts.push(Artifact::from_seed(&ArtifactSeed {
        id: "codex2".to_owned(),
        name: "Codex".to_owned(),
        focus: ArtifactFocus::Knowledge,
        power: 100,
        instability: 0.0,
        region_id: base_id.clone(),
    }));

    for _ in 0..200 {
        world.regions[0].chaos = 95.0;
        world.regions[0].danger = 95.0;
        world.regions[0].refresh_status(&data.balance.region);
        tick_genesis(&mut world, &data);
    }

    let rift_prefix = format!("{base_id}-rift-");
    assert!(
        !world.regions.iter().any(|r| r.id.starts_with(&rift_prefix)),
        "a region held by a Knowledge relic still fractured"
    );
    let region0 = world.regions.iter().find(|r| r.id == base_id).unwrap();
    assert!(
        region0.strife < data.balance.genesis.fracture_threshold,
        "the relic should keep strife below the fracture threshold"
    );
}

#[test]
fn calm_region_never_reshapes() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let start = world.regions.len();
    // Peaceful, not thriving: no crisis (no fracture/conquest) and short of
    // the thriving bar (no frontier).
    for _ in 0..300 {
        for region in &mut world.regions {
            region.chaos = 20.0;
            region.danger = 20.0;
            region.prosperity = 60.0;
            region.refresh_status(&data.balance.region);
        }
        tick_genesis(&mut world, &data);
    }
    assert_eq!(world.regions.len(), start, "a calm world reshaped itself");
}

#[test]
fn turmoil_without_a_leader_only_builds_pressure() {
    let data = GameData::load().unwrap();
    let mut world = primed_world(&data);
    // Strip every hero of the level that would lead a revolt OR defend a
    // region, and depress the would-be aggressors so conquest cannot fire —
    // proving pressure builds with no genesis event.
    for hero in &mut world.heroes {
        hero.level = 1;
    }
    for region in world.regions.iter_mut().skip(1) {
        region.prosperity = 10.0;
        region.population = 100.0;
    }
    let start = world.regions.len();
    for _ in 0..200 {
        world.regions[0].chaos = 95.0;
        world.regions[0].danger = 95.0;
        world.regions[0].refresh_status(&data.balance.region);
        tick_genesis(&mut world, &data);
    }
    assert_eq!(
        world.regions.len(),
        start,
        "leaderless region still reshaped"
    );
    assert!(
        world.regions[0].strife >= data.balance.genesis.fracture_threshold,
        "pressure should have kept building without a founder"
    );
    assert!(world.regions[0].status.is_crisis());
}

#[test]
fn a_strong_region_conquers_a_defenceless_neighbour() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let start = world.regions.len();

    // aldermoor (idx 0) is trade-linked to kharzul (idx 1).
    let loser_id = world.regions[0].id.clone();
    let winner_id = world.regions[1].id.clone();

    let winner = &mut world.regions[1];
    winner.prosperity = 90.0;
    winner.population = 40000.0;
    winner.chaos = 20.0;
    winner.danger = 20.0;
    winner.refresh_status(&data.balance.region);

    let loser = &mut world.regions[0];
    loser.prosperity = 8.0;
    loser.chaos = 90.0;
    loser.danger = 90.0;
    loser.population = 3000.0;
    loser.refresh_status(&data.balance.region);
    assert!(world.regions[0].status.is_crisis());
    for hero in &mut world.heroes {
        if hero.region_id == loser_id {
            hero.level = 1;
        }
    }
    // Strip the seeded Protection ward so this tests conquest in isolation.
    world.artifacts.retain(|a| a.region_id != loser_id);

    tick_genesis(&mut world, &data);

    assert_eq!(world.regions.len(), start - 1, "no region was conquered");
    assert!(
        !world.regions.iter().any(|r| r.id == loser_id),
        "the conquered region still exists"
    );
    assert!(
        world
            .settlements
            .iter()
            .filter(|s| s.region_id == winner_id)
            .count()
            >= 1
    );
    assert!(world
        .chronicle
        .iter_newest()
        .any(|e| e.message.contains("absorbs it whole")));
    // The conquest fed the world's momentum (drives Conquest-era pressure).
    assert!(world.conquest_momentum > 0.0);
}

#[test]
fn conquest_sacks_the_losers_greatest_city() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);

    // Same dominant-aggressor / defenceless-crisis setup that conquers.
    let loser_id = world.regions[0].id.clone();
    let winner = &mut world.regions[1];
    winner.prosperity = 90.0;
    winner.population = 40000.0;
    winner.chaos = 20.0;
    winner.danger = 20.0;
    winner.refresh_status(&data.balance.region);
    let loser = &mut world.regions[0];
    loser.prosperity = 8.0;
    loser.chaos = 90.0;
    loser.danger = 90.0;
    loser.population = 3000.0;
    loser.refresh_status(&data.balance.region);
    for hero in &mut world.heroes {
        if hero.region_id == loser_id {
            hero.level = 1;
        }
    }
    world.artifacts.retain(|a| a.region_id != loser_id);

    // The loser's greatest city, tracked by id (it changes hands, not identity).
    let (city_id, before_pop) = world
        .settlements
        .iter()
        .filter(|s| s.region_id == loser_id)
        .max_by(|a, b| a.population.total_cmp(&b.population))
        .map(|s| (s.id.clone(), s.population))
        .expect("the loser holds at least one settlement");

    tick_genesis(&mut world, &data);

    let sacked = world
        .settlements
        .iter()
        .find(|s| s.id == city_id)
        .expect("the sacked city survives the fall, diminished");
    assert!(
        sacked.population < before_pop * (1.0 - data.balance.conquest.sack_population_loss) + 1.0,
        "the greatest city should lose its people in the sack: {} -> {}",
        before_pop,
        sacked.population
    );
    assert!(
        world
            .chronicle
            .iter_newest()
            .any(|e| e.message.contains("is sacked")),
        "the sack of the great city should be chronicled"
    );
}

#[test]
fn a_defended_region_resists_conquest() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let start = world.regions.len();
    let loser_id = world.regions[0].id.clone();

    let winner = &mut world.regions[1];
    winner.prosperity = 90.0;
    winner.population = 40000.0;
    winner.chaos = 20.0;
    winner.danger = 20.0;
    winner.refresh_status(&data.balance.region);

    let loser = &mut world.regions[0];
    loser.prosperity = 8.0;
    loser.chaos = 90.0;
    loser.danger = 90.0;
    loser.population = 3000.0;
    loser.refresh_status(&data.balance.region);

    // A lone champion holds the line.
    world.heroes[0].region_id = loser_id.clone();
    world.heroes[0].level = 30;
    world.heroes[0].is_alive = true;

    for _ in 0..5 {
        tick_genesis(&mut world, &data);
    }
    assert_eq!(
        world.regions.len(),
        start,
        "a defended region was conquered anyway"
    );
    assert!(world.regions.iter().any(|r| r.id == loser_id));
}

#[test]
fn a_region_with_a_mighty_ally_resists_conquest() {
    // The same defenceless crisis region that would be annexed alone is spared
    // when a mighty ally is sworn to its defence — the ally lends the might, and
    // being an ally itself never falls upon a friend (GDD 5.2).
    use crate::world::Pact;

    // Whether the loser survives one genesis tick, with or without a sworn ally.
    let survives = |with_ally: bool| {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let loser_id = world.regions[0].id.clone();
        let ally_id = world.regions[2].id.clone();

        // Winner (kharzul) can annex; loser (aldermoor) in crisis and undefended.
        let winner = &mut world.regions[1];
        winner.prosperity = 90.0;
        winner.population = 40000.0;
        winner.chaos = 20.0;
        winner.danger = 20.0;
        winner.refresh_status(&data.balance.region);
        let loser = &mut world.regions[0];
        loser.prosperity = 8.0;
        loser.chaos = 90.0;
        loser.danger = 90.0;
        loser.population = 3000.0;
        loser.refresh_status(&data.balance.region);
        for hero in &mut world.heroes {
            if hero.region_id == loser_id {
                hero.level = 1;
            }
        }
        world.artifacts.retain(|a| a.region_id != loser_id);

        // A mighty, populous ally sworn to the loser marches to its defence.
        let ally = world.regions.iter_mut().find(|r| r.id == ally_id).unwrap();
        ally.prosperity = 100.0;
        ally.population = 200000.0;
        ally.refresh_status(&data.balance.region);
        if with_ally {
            world.pacts.push(Pact {
                id: "p".to_owned(),
                region_a: loser_id.clone(),
                region_b: ally_id,
                age: 5,
            });
        }

        tick_genesis(&mut world, &data);
        world.regions.iter().any(|r| r.id == loser_id)
    };

    assert!(
        !survives(false),
        "undefended and friendless, the crisis region should be annexed"
    );
    assert!(
        survives(true),
        "a mighty ally's aid should turn back the conquest"
    );
}

#[test]
fn a_protection_ward_turns_back_conquest() {
    use crate::data::{ArtifactFocus, ArtifactSeed};
    use crate::world::Artifact;
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let start = world.regions.len();
    let loser_id = world.regions[0].id.clone();

    // The same dominant-power-vs-defenceless-crisis setup that otherwise
    // conquers, but the loser is undefended by heroes.
    let winner = &mut world.regions[1];
    winner.prosperity = 90.0;
    winner.population = 40000.0;
    winner.chaos = 20.0;
    winner.danger = 20.0;
    winner.refresh_status(&data.balance.region);
    let loser = &mut world.regions[0];
    loser.prosperity = 8.0;
    loser.chaos = 90.0;
    loser.danger = 90.0;
    loser.population = 3000.0;
    loser.refresh_status(&data.balance.region);
    for hero in &mut world.heroes {
        if hero.region_id == loser_id {
            hero.level = 1;
        }
    }

    // A Protection ward of sufficient power stands over the doomed region.
    world.artifacts.push(Artifact::from_seed(&ArtifactSeed {
        id: "aegis".to_owned(),
        name: "Aegis".to_owned(),
        focus: ArtifactFocus::Protection,
        power: data.balance.conquest.shield_min_power,
        instability: 0.0,
        region_id: loser_id.clone(),
    }));

    for _ in 0..5 {
        tick_genesis(&mut world, &data);
    }
    assert_eq!(world.regions.len(), start, "a warded region was conquered");
    assert!(world.regions.iter().any(|r| r.id == loser_id));
}

#[test]
fn a_war_relic_empowers_a_marginal_conqueror() {
    use crate::data::{ArtifactFocus, ArtifactSeed};
    use crate::world::Artifact;
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    world.artifacts.clear(); // drop the seeded aegis ward on aldermoor
    let loser_id = world.regions[0].id.clone(); // aldermoor
    let winner_id = world.regions[1].id.clone(); // kharzul, trade-linked
    let start = world.regions.len();

    // An aggressor mighty enough to attack only once a war relic empowers it.
    {
        let w = &mut world.regions[1];
        w.culture = Culture::Mercantile; // no martial might bonus
        w.prosperity = 60.0;
        w.population = 5000.0;
        w.danger = 20.0;
        w.chaos = 20.0;
        w.refresh_status(&data.balance.region);
    }
    {
        let l = &mut world.regions[0];
        l.prosperity = 8.0;
        l.chaos = 90.0;
        l.danger = 90.0;
        l.population = 3000.0;
        l.refresh_status(&data.balance.region);
    }
    // Neutralise every other region: benign (not in crisis, so not a target)
    // and low-might (not an aggressor), leaving kharzul the only would-be
    // conqueror and aldermoor its only prey.
    for i in 2..world.regions.len() {
        let r = &mut world.regions[i];
        r.prosperity = 50.0;
        r.chaos = 20.0;
        r.danger = 20.0;
        r.population = 1000.0;
        r.refresh_status(&data.balance.region);
    }
    // Level the resident heroes of both the aggressor and its prey to 1, so the
    // aggressor's might comes from its stats and the relic under test — not from
    // strong heroes it happens to host, which lend their own conquest might.
    for hero in &mut world.heroes {
        if hero.region_id == loser_id || hero.region_id == winner_id {
            hero.level = 1;
        }
    }

    // Without a relic the aggressor is below the might floor — nothing happens.
    tick_genesis(&mut world, &data);
    assert_eq!(
        world.regions.len(),
        start,
        "a sub-threshold aggressor conquered without a relic"
    );

    // A War relic tips it over the threshold and the conquest lands.
    world.artifacts.push(Artifact::from_seed(&ArtifactSeed {
        id: "warhorn2".to_owned(),
        name: "Warhorn".to_owned(),
        focus: ArtifactFocus::War,
        power: 3,
        instability: 0.0,
        region_id: winner_id.clone(),
    }));
    tick_genesis(&mut world, &data);
    assert_eq!(
        world.regions.len(),
        start - 1,
        "the war relic did not empower the conquest"
    );
    assert!(!world.regions.iter().any(|r| r.id == loser_id));
}

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
