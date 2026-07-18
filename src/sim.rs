//! World tick orchestration. The server would own this in the multiplayer
//! design (GDD 7.1); in this local build the client runs it on a timer.

mod region;

use crate::data::GameConfig;
use crate::world::{EventKind, PlayerState, WorldState};

/// Advance the entire world by one tick: age every region, credit passive
/// favor, and record the chronicle entries a returning player would read.
pub fn tick_world(world: &mut WorldState, player: &mut PlayerState, config: &GameConfig) {
    world.year += 1;
    world.tick_count += 1;

    let mut newly_in_crisis: Vec<String> = Vec::new();
    for region in &mut world.regions {
        let was_crisis = region.status.is_crisis();
        region::tick_region(region);
        if region.status.is_crisis() && !was_crisis {
            newly_in_crisis.push(region.name.clone());
        }
    }

    player.recover(config);

    world.chronicle.push(
        world.year,
        EventKind::Tick,
        format!(
            "Year {} dawns. Favor +{}.",
            world.year, config.favor_per_tick
        ),
    );
    for name in newly_in_crisis {
        world.chronicle.push(
            world.year,
            EventKind::Region,
            format!("{name} has slipped into crisis."),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;

    #[test]
    fn tick_advances_year_and_favor() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut player = PlayerState::new(&data.config);
        player.favor = 0;
        let start_year = world.year;

        tick_world(&mut world, &mut player, &data.config);

        assert_eq!(world.year, start_year + 1);
        assert_eq!(world.tick_count, 1);
        assert_eq!(player.favor, data.config.favor_per_tick);
    }
}
