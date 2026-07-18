//! Local persistence of the simulated world.
//!
//! In the multiplayer design the server database is the save (GDD 8); this
//! local build persists the whole world through `macroquad_toolkit::persistence`
//! so the single-world client is a real, resumable game.

use crate::data::GameConfig;
use crate::world::{PlayerState, WorldState};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub version: String,
    pub world: WorldState,
    pub player: PlayerState,
}

impl SaveData {
    pub fn new(world: &WorldState, player: &PlayerState, version: &str) -> Self {
        Self {
            version: version.to_owned(),
            world: world.clone(),
            player: player.clone(),
        }
    }
}

/// Migration hook for the toolkit loader. Accepts the current shape and stamps
/// the running version; unknown shapes are a hard error rather than a silent
/// reset, matching the fail-fast discipline used across the codebase.
pub fn migrate_save_value(
    detected_version: Option<String>,
    value: Value,
    config: &GameConfig,
) -> Result<SaveData, String> {
    let payload = value.get("data").cloned().unwrap_or(value);
    let mut save: SaveData = serde_json::from_value(payload)
        .map_err(|err| format!("Unsupported save {:?}: {}", detected_version, err))?;
    save.version = config.version.clone();
    Ok(save)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::sim::tick_world;

    #[test]
    fn save_load_preserves_deterministic_continuation() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut player = PlayerState::new(&data.config);
        // Advance a while so the RNG state and rosters diverge from the seed.
        for _ in 0..30 {
            tick_world(&mut world, &mut player, &data);
        }

        // Round-trip through JSON exactly as the persistence layer does.
        let save = SaveData::new(&world, &player, &data.config.version);
        let json = serde_json::to_string(&save).unwrap();
        let loaded: SaveData = serde_json::from_str(&json).unwrap();

        let (mut world_a, mut player_a) = (world, player);
        let (mut world_b, mut player_b) = (loaded.world, loaded.player);

        // A correct save carries the whole simulation state — including the RNG —
        // so continued play stays in lockstep with an unsaved world.
        for _ in 0..30 {
            tick_world(&mut world_a, &mut player_a, &data);
            tick_world(&mut world_b, &mut player_b, &data);
        }

        assert_eq!(world_a.year, world_b.year);
        assert_eq!(
            serde_json::to_string(&world_a).unwrap(),
            serde_json::to_string(&world_b).unwrap(),
            "loaded world diverged — some sim state is not persisted"
        );
        assert_eq!(
            serde_json::to_string(&player_a).unwrap(),
            serde_json::to_string(&player_b).unwrap(),
            "loaded player diverged — some player state is not persisted"
        );
    }
}
