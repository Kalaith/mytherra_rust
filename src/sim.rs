//! World tick orchestration. The server would own this in the multiplayer
//! design (GDD 7.1); in this local build the client runs it on a timer.

mod artifact;
mod champion;
mod civilization;
mod era;
mod hero;
mod magic;
mod myth;
mod pantheon;
mod region;
mod resource;
mod settlement;
mod speculation;
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
        let was_crisis = region.status.is_crisis();
        region::tick_region(region, &data.balance.region);
        if region.status.is_crisis() && !was_crisis {
            newly_in_crisis.push(region.name.clone());
        }
    }

    settlement::tick_settlements(
        &mut world.settlements,
        &mut world.regions,
        &data.balance.settlement,
        &data.balance.region,
    );

    resource::tick_resources(
        &mut world.resource_nodes,
        &mut world.regions,
        &mut world.rng,
        &data.balance.resource,
        &data.balance.region,
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
        &data.balance.weather,
        &data.balance.region,
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
}
