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
