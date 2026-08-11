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

mod test_part_1;
mod test_part_2;
