//! Static game configuration and tuning values loaded from JSON.

use serde::{Deserialize, Serialize};

/// Top-level tuning for the world simulation and the divine-favor economy.
///
/// Env vars must never have code defaults elsewhere; this mirrors the same
/// discipline for content — every value here comes from `game_config.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub game_name: String,
    pub display_name: String,
    pub save_slot: String,
    pub version: String,
    /// Divine Favor granted to a new player (see GDD 5.1).
    pub starting_favor: i64,
    /// Passive favor recovery credited each world tick.
    pub favor_per_tick: i64,
    /// Hard ceiling on a player's favor balance.
    pub max_favor: i64,
    /// Real seconds between automatic world ticks.
    pub seconds_per_tick: f32,
    /// Calendar year the world begins on.
    pub start_year: u32,
    /// Seed for the world's deterministic simulation RNG (GDD 5.8).
    pub world_seed: u64,
}
