//! Magic content types: the five research paths and the region stat each one
//! influences once unlocked (GDD 5.6).

use serde::{Deserialize, Serialize};

/// Which region stat a magic path nudges when emerging/known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MagicStat {
    Prosperity,
    Chaos,
    Danger,
    Magic,
}

/// A seeded research path (`magic_paths.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagicPathSeed {
    pub id: String,
    pub name: String,
    pub description: String,
    pub effect_stat: MagicStat,
    /// Per-tick stat delta applied to every region once known (scaled while
    /// merely emerging); sign carried by the value.
    pub effect_per_tick: f32,
}
