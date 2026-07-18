//! Simulation and economy tuning, loaded from `balance.json`.
//!
//! Every magic number the world sim and favor economy use lives here rather
//! than in Rust source, per the data-driven design rule. Rust only names the
//! shape; designers tune the values in JSON.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub region: RegionBalance,
    pub player: PlayerBalance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionBalance {
    pub cost_multiplier: MultiplierCurve,
    pub effect_multiplier: MultiplierCurve,
    pub status: StatusThresholds,
    pub drift: DriftParams,
}

/// A resonance-scaled multiplier: `clamp(min, max, 1 +/- (resonance-50) * coeff)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiplierCurve {
    pub coeff: f32,
    pub min: f32,
    pub max: f32,
}

/// Thresholds that derive a region's status band from its stats (GDD 5.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusThresholds {
    pub wartorn_danger: f32,
    pub wartorn_chaos: f32,
    pub unrest_chaos: f32,
    pub thriving_prosperity: f32,
    pub thriving_chaos_max: f32,
    pub prospering_prosperity: f32,
    pub struggling_prosperity: f32,
}

/// Per-tick region drift parameters (GDD 5.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftParams {
    pub high_chaos_threshold: f32,
    pub low_chaos_threshold: f32,
    pub prosperity_high_chaos: f32,
    pub prosperity_low_chaos: f32,
    pub prosperity_mid: f32,
    pub chaos_target: f32,
    pub chaos_rate: f32,
    pub danger_target: f32,
    pub danger_rate: f32,
    pub magic_target: f32,
    pub magic_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerBalance {
    pub level_base_cost: i64,
    pub level_cost_step: i64,
}
