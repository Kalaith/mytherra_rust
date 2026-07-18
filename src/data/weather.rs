//! Weather content types: patterns and intensities (GDD 5.6).

use serde::{Deserialize, Serialize};

/// A weather pattern's per-magnitude-unit effect on a region's stats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherPattern {
    pub id: String,
    pub name: String,
    pub prosperity: f32,
    pub chaos: f32,
    pub danger: f32,
    pub magic: f32,
}

/// A weather intensity: how strong (and how costly) a shaped pattern is.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherIntensity {
    pub id: String,
    pub name: String,
    pub magnitude: f32,
    pub cost_mult: f32,
}
