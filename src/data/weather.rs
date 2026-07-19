//! Weather content types: patterns and intensities (GDD 5.6).

use crate::data::ClimateType;
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
    /// Climates that naturally give rise to this pattern (GDD 5.6). Natural
    /// weather over a region is drawn from the patterns matching its climate;
    /// the player can still shape any pattern anywhere.
    #[serde(default)]
    pub climates: Vec<ClimateType>,
}

/// A weather intensity: how strong (and how costly) a shaped pattern is.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherIntensity {
    pub id: String,
    pub name: String,
    pub magnitude: f32,
    pub cost_mult: f32,
}
