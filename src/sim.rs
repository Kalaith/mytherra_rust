//! World tick orchestration. The server would own this in the multiplayer
//! design (GDD 7.1); in this local build the client runs it on a timer.

mod artifact;
mod champion;
mod civilization;
mod consequence;
mod culture;
mod era;
mod genesis;
mod hero;
mod landmark;
mod magic;
mod myth;
mod pantheon;
mod region;
mod resource;
mod settlement;
mod speculation;
mod trade;
mod weather;

use crate::data::{fill, GameData};
use crate::world::{EventKind, Hero, PlayerState, WorldState};

/// Advance the entire world by one tick: age every region, credit passive
/// favor, and record the chronicle entries a returning player would read.
pub fn tick_world(world: &mut WorldState, player: &mut PlayerState, data: &GameData) {
    world.year += 1;
    world.tick_count += 1;

    // Heroes who were already legends (the top renown title) before this tick, so
    // we can chronicle the moment any hero first crosses that bar — a milestone
    // the level-up, era-survival, and mastered-magic renown systems all feed.
    let legend_bar = data
        .balance
        .hero
        .renown
        .thresholds
        .last()
        .copied()
        .unwrap_or(f32::INFINITY);
    let already_legend: Vec<String> = world
        .heroes
        .iter()
        .filter(|h| h.renown >= legend_bar)
        .map(|h| h.id.clone())
        .collect();

    let mut newly_in_crisis: Vec<String> = Vec::new();
    for region in &mut world.regions {
        // Baseline for this tick's trend arrows, before any system moves stats.
        region.snapshot_trend();
        let was_crisis = region.status.is_crisis();
        region::tick_region(region, &data.balance.region);
        if region.status.is_crisis() && !was_crisis {
            newly_in_crisis.push(region.name.clone());
        }
    }

    settlement::tick_settlements(
        &mut world.settlements,
        &world.buildings,
        &mut world.regions,
        &data.balance.settlement,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        &data.strings.ui.settlement_tiers,
        world.year,
    );

    settlement::tick_settlement_abandonment(
        &mut world.settlements,
        &mut world.buildings,
        &data.balance.settlement,
        &world.regions,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    settlement::tick_settlement_founding(
        &mut world.settlements,
        &world.regions,
        &mut world.settlement_seq,
        &data.settlement_names,
        &data.balance.settlement,
        &mut world.rng,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    settlement::tick_construction(
        &world.settlements,
        &world.regions,
        &mut world.buildings,
        &data.building_types,
        &data.balance.settlement,
        &mut world.rng,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    resource::tick_resources(
        &mut world.resource_nodes,
        &mut world.regions,
        &mut world.rng,
        &data.balance.resource,
        &data.balance.region,
    );

    trade::tick_trade(
        &world.trade_routes,
        &mut world.regions,
        &data.balance.trade,
        &data.balance.region,
    );

    culture::tick_culture(
        &mut world.regions,
        &world.heroes,
        &world.landmarks,
        &world.resource_nodes,
        &world.settlements,
        &world.buildings,
        &world.trade_routes,
        &data.balance.culture,
        &data.balance.region,
        &data.balance.settlement.tier_thresholds,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    landmark::tick_landmark_founding(
        &mut world.landmarks,
        &world.regions,
        &mut world.landmark_seq,
        &data.landmark_names,
        &data.balance.culture,
        &mut world.rng,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    hero::tick_heroes(
        &mut world.heroes,
        &world.regions,
        &world.landmarks,
        &mut world.rng,
        &data.balance.hero,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    champion::tick_champions(
        &mut player.champions,
        &mut world.heroes,
        &mut world.regions,
        &data.balance.champion,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    artifact::tick_artifacts(
        &mut world.artifacts,
        &mut world.regions,
        &mut world.pending_consequences,
        &data.balance.artifact,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    // Delayed aftermath steps of past backlashes unfold here (GDD 5.6).
    consequence::tick_consequences(
        &mut world.pending_consequences,
        &mut world.regions,
        &mut world.settlements,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    weather::tick_weather(
        &mut world.weather,
        &mut world.regions,
        &data.weather_patterns,
        &data.weather_intensities,
        &mut world.rng,
        &data.balance.weather,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    magic::tick_magic(
        &mut world.magic_paths,
        &mut world.regions,
        &mut world.heroes,
        &data.balance.magic,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    myth::tick_myths(
        &mut world.myths,
        &mut world.myth_candidates,
        &mut world.myth_seq,
        &mut world.regions,
        &mut world.rng,
        &mut world.chronicle,
        data,
        world.year,
    );

    civilization::tick_civilization(
        &mut world.civilization,
        &mut world.regions,
        &data.agendas,
        &data.balance.civilization,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    // With every stat-mover settled for this tick, let the map reshape: regions
    // pushed past breaking fracture into new ones, and strong powers annex
    // collapsed, undefended neighbours (GDD 5.2).
    genesis::tick_genesis(world, data);

    // Snapshot deity tiers so we can chronicle any god that crests into wrath
    // this tick — the pantheon's autonomous stirring is otherwise silent.
    let deity_tiers: Vec<usize> = world
        .pantheon
        .iter()
        .map(|d| d.tier(&data.balance.pantheon))
        .collect();
    pantheon::tick_pantheon(
        &mut world.pantheon,
        &mut world.regions,
        &data.balance.pantheon,
        &data.balance.region,
    );
    for name in pantheon::deities_cresting(&deity_tiers, &world.pantheon, &data.balance.pantheon) {
        world.chronicle.push(
            world.year,
            EventKind::Divine,
            fill(&data.strings.chronicle.deity_ascendant, &[("deity", name)]),
        );
    }

    let era_progress = world.era.pressure / data.balance.era.breaking_threshold.max(1.0);
    speculation::tick_speculations(
        &mut world.speculations,
        &mut world.speculation_seq,
        player,
        &world.heroes,
        &world.regions,
        &world.settlements,
        &mut world.chronicle,
        &mut world.rng,
        data,
        world.year,
        world.era.number,
        era_progress,
    );

    era::tick_era(world, player, data);

    player.recover(&data.config, &data.balance.player);

    // The chronicle records notable events, not the passing of each year — the
    // year and favor already live in the HUD, so no per-tick heartbeat clutters
    // the Event Log and drowns the deity's own actions (GDD 10).
    let text = &data.strings.chronicle;
    for name in newly_in_crisis {
        world.chronicle.push(
            world.year,
            EventKind::Region,
            fill(&text.crisis, &[("region", name)]),
        );
    }
    // A hero crossing into legend is both chronicled and — since a legend needs
    // no promotion to be told — seeded as a myth candidate about them.
    let new_legends: Vec<(String, String, String)> =
        newly_legendary(&already_legend, &world.heroes, legend_bar)
            .into_iter()
            .map(|h| {
                let region_name = world
                    .regions
                    .iter()
                    .find(|r| r.id == h.region_id)
                    .map(|r| r.name.clone())
                    .unwrap_or_default();
                (h.name.clone(), h.region_id.clone(), region_name)
            })
            .collect();
    for (name, region_id, region_name) in new_legends {
        world.chronicle.push(
            world.year,
            EventKind::Hero,
            fill(&text.hero_legend, &[("hero", name.clone())]),
        );
        myth::seed_hero_legend(
            &mut world.myth_candidates,
            &mut world.myth_seq,
            &name,
            &region_id,
            &region_name,
            data,
        );
    }
}

/// Living heroes who have reached `bar` renown this tick but hadn't before, so
/// the crossing into legend is handled exactly once.
fn newly_legendary<'a>(before: &[String], heroes: &'a [Hero], bar: f32) -> Vec<&'a Hero> {
    heroes
        .iter()
        .filter(|h| h.is_alive && h.renown >= bar && !before.iter().any(|id| id == &h.id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn prosperity_settles_into_a_dynamic_range() {
        // With mean-reverting drift, a long unmanaged run should neither pin
        // every region at the ceiling nor collapse the whole world.
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut player = PlayerState::new(&data.config);
        for _ in 0..250 {
            tick_world(&mut world, &mut player, &data);
        }
        let summary = world.summary();
        assert!(
            summary.avg_prosperity < 92.0,
            "prosperity pinned: {}",
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
}
