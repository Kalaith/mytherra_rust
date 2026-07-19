//! World tick orchestration. The server would own this in the multiplayer
//! design (GDD 7.1); in this local build the client runs it on a timer.

mod artifact;
mod champion;
mod civilization;
mod culture;
mod era;
mod genesis;
mod hero;
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
use crate::world::{EventKind, PlayerState, WorldState};

/// Advance the entire world by one tick: age every region, credit passive
/// favor, and record the chronicle entries a returning player would read.
pub fn tick_world(world: &mut WorldState, player: &mut PlayerState, data: &GameData) {
    world.year += 1;
    world.tick_count += 1;

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
    );

    settlement::tick_construction(
        &world.settlements,
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
        &world.trade_routes,
        &data.balance.culture,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    hero::tick_heroes(
        &mut world.heroes,
        &world.regions,
        &mut world.rng,
        &data.balance.hero,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    champion::tick_champions(
        &mut player.champions,
        &world.heroes,
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
        &data.balance.artifact,
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
    );

    // With every stat-mover settled for this tick, let the map reshape: regions
    // pushed past breaking fracture into new ones, and strong powers annex
    // collapsed, undefended neighbours (GDD 5.2).
    genesis::tick_genesis(world, data);

    pantheon::tick_pantheon(
        &mut world.pantheon,
        &mut world.regions,
        &data.balance.pantheon,
        &data.balance.region,
    );

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
    );

    era::tick_era(world, player, data);

    player.recover(&data.config);

    let text = &data.strings.chronicle;
    world.chronicle.push(
        world.year,
        EventKind::Tick,
        fill(
            &text.year_dawns,
            &[
                ("year", world.year.to_string()),
                ("favor", data.config.favor_per_tick.to_string()),
            ],
        ),
    );
    for name in newly_in_crisis {
        world.chronicle.push(
            world.year,
            EventKind::Region,
            fill(&text.crisis, &[("region", name)]),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        for hero in &mut world.heroes {
            if hero.region_id == loser_id {
                hero.level = 1;
            }
        }
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
        world.heroes[0].region_id = doomed.clone();
        world.heroes[0].level = 20;

        let start = world.regions.len();
        let mut fractured = false;
        for _ in 0..400 {
            // Keep the region violently unstable, the way relentless corruption
            // or a divine-war era would; drift alone would otherwise calm it.
            if let Some(r) = world.regions.iter_mut().find(|r| r.id == doomed) {
                r.chaos = 95.0;
                r.danger = 95.0;
            }
            tick_world(&mut world, &mut player, &data);
            if world.regions.len() > start {
                fractured = true;
                break;
            }
        }
        assert!(fractured, "sustained turmoil never fractured the region");
        assert!(
            world
                .chronicle
                .iter_newest()
                .any(|e| e.message.contains("revolt")),
            "the fracture was not chronicled"
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
}
