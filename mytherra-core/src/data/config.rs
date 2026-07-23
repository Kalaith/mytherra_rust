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
    /// Autosave the world every N ticks (0 disables autosave).
    pub autosave_every_ticks: u64,
    /// Calendar year the world begins on.
    pub start_year: u32,
    /// Seed for the world's deterministic simulation RNG (GDD 5.8).
    pub world_seed: u64,
    /// Base URL of the authority server the client connects to (GDD 7.4). The
    /// client is online-only — there is no local-world play — so this must point
    /// at a running `mytherra-server`.
    pub server_url: String,
    /// Address the authority server binds its listener to (GDD 7.6). Kept in
    /// config so the deployment address lives in one place, not a source const.
    pub server_listen_addr: String,
    /// Real seconds between the online client's `GET /view` polls. The shared
    /// world turns on the server's schedule; the client re-fetches its
    /// projection at this cadence (and immediately after each submitted action).
    pub view_poll_seconds: f32,
}
