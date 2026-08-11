use super::*;

#[test]
fn faithful_lands_tithe_favor_and_faithless_ones_do_not() {
    let data = GameData::load().unwrap();
    let balance = &data.balance.player;
    let mut world = WorldState::new(&data);
    world.regions.truncate(2);

    // Both at the neutral baseline: no land is faithful, so no tithe.
    for r in &mut world.regions {
        r.divine_resonance = balance.favor_tithe_baseline;
    }
    assert_eq!(
        faith_tithe(&world.regions, balance),
        0,
        "lands at the baseline tithe nothing"
    );

    // One hallowed land pours favor back; a faithless (below-baseline) one adds
    // nothing, never a negative.
    world.regions[0].divine_resonance = balance.favor_tithe_baseline + 100.0;
    world.regions[1].divine_resonance = balance.favor_tithe_baseline - 30.0;
    let expected = (100.0 * balance.favor_per_resonance) as i64;
    assert_eq!(
        faith_tithe(&world.regions, balance),
        expected,
        "only resonance above the baseline tithes, and never below zero"
    );
    assert!(expected > 0, "a hallowed land should tithe real favor");
}

#[test]
fn only_a_fresh_living_crossing_into_legend_is_reported() {
    use crate::data::HeroRole;
    let h = |id: &str, renown: f32, alive: bool| Hero {
        id: id.to_owned(),
        name: format!("{id}-name"),
        role: HeroRole::Warrior,
        region_id: "r".to_owned(),
        level: 1,
        age: 20,
        is_alive: alive,
        renown,
    };
    let heroes = vec![
        h("old", 200.0, true),   // already a legend last tick — no repeat
        h("new", 200.0, true),   // crossed this tick — announced
        h("dead", 200.0, false), // legendary but fallen — no fanfare
        h("mortal", 50.0, true), // below the bar
    ];
    let before = vec!["old".to_owned()];
    let crossed = newly_legendary(&before, &heroes, 180.0);
    let names: Vec<&str> = crossed.iter().map(|h| h.name.as_str()).collect();
    assert_eq!(names, vec!["new-name"]);
}

#[test]
fn tick_advances_year_and_favor() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut player = PlayerState::new(&data.config);
    player.favor = 0;
    let start_year = world.year;

    tick_world(&mut world, &mut player, &data);

    assert_eq!(world.year, start_year + 1);
    assert_eq!(world.tick_count, 1);
    assert_eq!(player.favor, data.config.favor_per_tick);
}

#[test]
fn a_collapsed_region_is_conquered_through_the_full_tick() {
    // End-to-end: with a dominant power next door and no defender, a
    // crisis-stricken region is annexed — and every later tick system copes
    // with the region vanishing mid-tick. This is the integration guard that
    // removing a region from `world.regions` never desyncs the pipeline.
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut player = PlayerState::new(&data.config);
    let start = world.regions.len();

    let loser_id = world.regions[0].id.clone(); // aldermoor, trade-linked to kharzul
    world.regions[1].prosperity = 90.0;
    world.regions[1].population = 40000.0;
    world.regions[1].chaos = 20.0;
    world.regions[1].danger = 20.0;
    world.regions[1].refresh_status(&data.balance.region);
    world.regions[0].prosperity = 8.0;
    world.regions[0].chaos = 90.0;
    world.regions[0].danger = 90.0;
    world.regions[0].population = 3000.0;
    world.regions[0].refresh_status(&data.balance.region);
    // Remove every hero so no defender can arise — and so this determinism
    // guard stays robust as the seeded roster changes over time (conquest
    // itself uses no RNG; it fires purely on the region state set up here).
    world.heroes.clear();
    // Strip the seeded Protection ward so this tests conquest in isolation.
    world.artifacts.retain(|a| a.region_id != loser_id);

    tick_world(&mut world, &mut player, &data);

    assert_eq!(world.regions.len(), start - 1, "no region was conquered");
    assert!(!world.regions.iter().any(|r| r.id == loser_id));
    assert!(world.summary().region_count == start - 1);
}

#[test]
fn a_region_ground_down_by_turmoil_fractures_through_the_full_tick() {
    // End-to-end: a region kept in crisis (as sustained divine corruption or
    // a long war-torn era would) should eventually split into a new region,
    // and the schism should reach the chronicle — all via `tick_world`.
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut player = PlayerState::new(&data.config);
    // Plant a capable would-be founder in the doomed region.
    let doomed = world.regions[0].id.clone();
    let founder = world.heroes[0].id.clone();
    world.heroes[0].region_id = doomed.clone();
    world.heroes[0].level = 20;

    let mut revolted = false;
    for _ in 0..400 {
        // Keep the region violently unstable, the way relentless corruption
        // or a divine-war era would; drift alone would otherwise calm it. Keep
        // its capable founder in place and alive too, so it always has someone
        // to lead the revolt and a defender against being conquered out from
        // under the test.
        if let Some(r) = world.regions.iter_mut().find(|r| r.id == doomed) {
            r.chaos = 95.0;
            r.danger = 95.0;
        }
        if let Some(h) = world.heroes.iter_mut().find(|h| h.id == founder) {
            h.region_id = doomed.clone();
            h.is_alive = true;
            h.level = 20;
        }
        tick_world(&mut world, &mut player, &data);
        // Break on the revolt itself — other genesis events (a frontier
        // founding elsewhere) may grow the map first, but the thing under test
        // is that sustained turmoil sparks a *revolt*.
        if world
            .chronicle
            .iter_newest()
            .any(|e| e.message.contains("revolt"))
        {
            revolted = true;
            break;
        }
    }
    assert!(
        revolted,
        "sustained turmoil never sparked a chronicled revolt"
    );
}

#[test]
fn the_hero_population_survives_the_ages() {
    // A 400-year unmanaged run must keep a living hero population across the
    // era cullings, and at least one hero must reach the top renown title (a
    // living legend) somewhere along the way — the champion/renown/legend web
    // depends on the roster never dwindling to nothing (GDD 5.4). Guards the
    // hero-lifecycle tuning against a regression that starves the world of
    // heroes.
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut player = PlayerState::new(&data.config);
    let bar = *data.balance.hero.renown.thresholds.last().unwrap();
    let mut min_alive = usize::MAX;
    let mut ever_legend = false;
    for _ in 0..400 {
        tick_world(&mut world, &mut player, &data);
        let living = world.heroes.iter().filter(|h| h.is_alive);
        min_alive = min_alive.min(living.clone().count());
        ever_legend |= living.map(|h| h.renown).fold(0.0_f32, f32::max) >= bar;
    }
    assert!(
        min_alive >= 2,
        "the hero roster dwindled to {min_alive} — the world starves of heroes"
    );
    assert!(ever_legend, "no hero ever rose to legend across four ages");
}

#[test]
fn prosperity_settles_into_a_dynamic_range() {
    // With mean-reverting drift, a long unmanaged run should neither climb
    // toward a static utopia (the positive systems stacking on the reversion)
    // nor collapse the whole world. The upper bound is deliberately tighter
    // than the 100 ceiling: it guards against the world re-drifting into a
    // crisis-free paradise as more prosperity-lifting systems are added.
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut player = PlayerState::new(&data.config);
    for _ in 0..250 {
        tick_world(&mut world, &mut player, &data);
    }
    let summary = world.summary();
    assert!(
        summary.avg_prosperity < 88.0,
        "prosperity drifting toward utopia: {}",
        summary.avg_prosperity
    );
    assert!(
        summary.avg_prosperity > 25.0,
        "world collapsed: {}",
        summary.avg_prosperity
    );
    assert!(
        summary.avg_magic < 92.0,
        "magic pinned: {}",
        summary.avg_magic
    );
}

#[test]
fn the_world_stays_coherent_across_many_ages() {
    // Drive the whole pipeline through several era transitions, asserting the
    // world never degenerates: every stat stays finite and in range, no NaN
    // slips in, settlements never go negative, and the map is never emptied.
    // A cross-system regression guard for the deterministic simulation.
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut player = PlayerState::new(&data.config);

    for _ in 0..350 {
        tick_world(&mut world, &mut player, &data);

        assert!(!world.regions.is_empty(), "the map was emptied of regions");
        for r in &world.regions {
            for v in [
                r.prosperity,
                r.chaos,
                r.danger,
                r.magic_affinity,
                r.cultural_influence,
                r.divine_resonance,
            ] {
                assert!(
                    v.is_finite() && (0.0..=100.0).contains(&v),
                    "region {} stat out of range: {v}",
                    r.id
                );
            }
            assert!(r.population.is_finite() && r.population >= 0.0);
            assert!(r.strife.is_finite() && r.strife >= 0.0);
        }
        for s in &world.settlements {
            assert!(
                s.prosperity.is_finite() && (0.0..=100.0).contains(&s.prosperity),
                "settlement {} prosperity out of range: {}",
                s.id,
                s.prosperity
            );
            assert!(s.population.is_finite() && s.population >= 0.0);
        }
        assert!(player.favor >= 0, "favor went negative");

        // Genesis must never mint two regions sharing a name (GDD 5.2).
        let mut names: Vec<&str> = world.regions.iter().map(|r| r.name.as_str()).collect();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(
            total,
            names.len(),
            "two regions share a name at year {}",
            world.year
        );
    }
    assert!(world.year >= 350);
}

#[test]
fn the_same_seed_yields_a_bit_identical_world() {
    // GDD 5.8: the simulation is fully deterministic — the same seed and the
    // same inputs must reproduce the exact same world, byte for byte, so any
    // outcome is auditable. Two independent runs are compared over their full
    // serialized state (not just a digest) after many ages.
    let data = GameData::load().unwrap();
    let run = || {
        let mut world = WorldState::new(&data);
        let mut player = PlayerState::new(&data.config);
        for _ in 0..200 {
            tick_world(&mut world, &mut player, &data);
        }
        (
            serde_json::to_string(&world).expect("world serializes"),
            serde_json::to_string(&player).expect("player serializes"),
        )
    };
    assert_eq!(
        run(),
        run(),
        "same seed must reproduce identical world and player state"
    );
}
