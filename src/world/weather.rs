//! Runtime weather state (GDD 5.6): a shaped weather front over a region that
//! applies its pattern each tick and decays until it dissipates.

use crate::data::{WeatherIntensity, WeatherPattern};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherEvent {
    pub region_id: String,
    pub pattern_id: String,
    pub pattern_name: String,
    pub intensity_name: String,
    /// Remaining strength; decays each tick until it drops below the floor.
    pub magnitude: f32,
    /// Per-magnitude-unit stat effects (denormalized from the pattern).
    pub prosperity: f32,
    pub chaos: f32,
    pub danger: f32,
    pub magic: f32,
}

impl WeatherEvent {
    /// Build a front over a region from a pattern and intensity — shared by the
    /// player's shape action and natural weather emergence.
    pub fn from_parts(
        region_id: String,
        pattern: &WeatherPattern,
        intensity: &WeatherIntensity,
    ) -> Self {
        Self {
            region_id,
            pattern_id: pattern.id.clone(),
            pattern_name: pattern.name.clone(),
            intensity_name: intensity.name.clone(),
            magnitude: intensity.magnitude,
            prosperity: pattern.prosperity,
            chaos: pattern.chaos,
            danger: pattern.danger,
            magic: pattern.magic,
        }
    }

    /// Whether this front blesses or blights the land it sits on: a net gain of
    /// the good stats (prosperity, magic) over the ill ones (chaos, danger), so a
    /// glance tells fair weather from foul (GDD 5.6). Shared by the Weather tool's
    /// front cards and the region detail's active-skies line.
    pub fn is_fair(&self) -> bool {
        self.prosperity + self.magic - self.chaos - self.danger > 0.0
    }
}

/// Favor cost to shape weather: base scaled by intensity and (inversely) by the
/// region's divine resonance, matching how region actions scale (GDD 5.6).
pub fn weather_cost(base_cost: i64, intensity_cost_mult: f32, region_cost_mult: f32) -> i64 {
    ((base_cost as f32 * intensity_cost_mult * region_cost_mult).round() as i64).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stronger_intensity_costs_more() {
        assert!(weather_cost(14, 3.0, 1.0) > weather_cost(14, 1.0, 1.0));
    }

    #[test]
    fn high_resonance_region_is_cheaper() {
        assert!(weather_cost(14, 1.0, 0.7) < weather_cost(14, 1.0, 1.3));
    }
}
