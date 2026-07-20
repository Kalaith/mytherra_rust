//! Region, culture, and trade tuning (GDD 5.2).

use crate::data::ClimateType;
use serde::{Deserialize, Serialize};

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

/// Per-tick region drift parameters (GDD 5.2). Prosperity mean-reverts toward a
/// chaos/danger-derived equilibrium so the world settles dynamically instead of
/// climbing to the ceiling as every system stacks positive contributions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftParams {
    pub prosperity_target_base: f32,
    pub prosperity_chaos_weight: f32,
    pub prosperity_danger_weight: f32,
    pub prosperity_reversion_rate: f32,
    pub chaos_target: f32,
    pub chaos_rate: f32,
    pub danger_target: f32,
    pub danger_rate: f32,
    /// Per-climate offset to the danger equilibrium (GDD 5.2): harsh lands
    /// (frozen, arid) settle more dangerous than hospitable ones, so an untended
    /// region keeps the character of its climate instead of every region
    /// relaxing to one shared baseline.
    pub climate_danger: ClimateDrift,
    pub magic_target: f32,
    /// Proportional pull toward `magic_target`, so magic — pushed up by
    /// knowledge artifacts / divination / the Growth deity — settles rather than
    /// pinning at the ceiling (mirrors the prosperity mean-reversion).
    pub magic_reversion_rate: f32,
}

/// A per-climate value, one field per `ClimateType`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClimateDrift {
    pub temperate: f32,
    pub arid: f32,
    pub tropical: f32,
    pub frozen: f32,
    pub coastal: f32,
    pub highland: f32,
}

impl ClimateDrift {
    pub fn danger_offset(&self, climate: ClimateType) -> f32 {
        match climate {
            ClimateType::Temperate => self.temperate,
            ClimateType::Arid => self.arid,
            ClimateType::Tropical => self.tropical,
            ClimateType::Frozen => self.frozen,
            ClimateType::Coastal => self.coastal,
            ClimateType::Highland => self.highland,
        }
    }
}

/// Dynamic culture-scoring tuning (GDD 5.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CultureBalance {
    /// A challenger must beat the incumbent by this to flip the dominant culture.
    pub inertia: f32,
    pub hero_weight: f32,
    pub landmark_weight: f32,
    pub resource_weight: f32,
    pub settlement_weight: f32,
    /// Mercantile score per trade route touching a region (weighted by volume).
    pub trade_weight: f32,
    /// Cultural-influence baseline and per-landmark bonus (the reversion target).
    pub influence_base: f32,
    pub influence_per_landmark: f32,
    pub influence_rate: f32,
}

/// Trade-route tuning (GDD 5.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeBalance {
    /// Prosperity added to each endpoint per tick, per unit of route volume.
    pub prosperity_bonus: f32,
    /// Fraction each endpoint drifts toward the pair's average prosperity.
    pub equalize_rate: f32,
    /// Cultural influence added to each endpoint per tick, per unit of volume:
    /// ideas travel the trade network alongside wealth (GDD 5.2).
    pub culture_bonus: f32,
    /// Fraction each endpoint drifts toward the pair's average cultural influence.
    pub culture_equalize: f32,
}
