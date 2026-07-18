//! Runtime weather state (GDD 5.6): a shaped weather front over a region that
//! applies its pattern each tick and decays until it dissipates.

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
