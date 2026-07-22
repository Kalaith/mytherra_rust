//! Region, culture, and trade tuning (GDD 5.2).

use crate::data::ClimateType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionBalance {
    pub cost_multiplier: MultiplierCurve,
    pub effect_multiplier: MultiplierCurve,
    /// Divine resonance a region gains each time the player acts on it directly
    /// (Bless/Corrupt/Guide) — a god's repeated touch consecrates the land (GDD
    /// 5.2), making it cheaper and more responsive to future nudges (and more
    /// keenly felt by a roused pantheon). Player-driven only; the world's own
    /// drift never touches resonance.
    pub resonance_per_action: f32,
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
    /// Culture score a building lends its region toward the culture it embodies
    /// (GDD 6 <-> 5.2): the works a people raise express and reinforce their
    /// character, so a land of forges hardens martial and one of temples turns
    /// mystical — a slow feedback the player can lean on or fight.
    pub building_weight: f32,
    /// How much a settlement's *size tier* amplifies its Mercantile pull (GDD
    /// 5.2): each tier above a hamlet adds this fraction, so a great city is a
    /// far stronger engine of commerce than a village of the same prosperity —
    /// urbanization erodes a region's older, rural identity. At 0 size is
    /// ignored (the old tier-blind behaviour).
    pub settlement_tier_weight: f32,
    /// Mercantile score per trade route touching a region (weighted by volume).
    pub trade_weight: f32,
    /// Culture score a living myth lends its home region, per myth, scaled by its
    /// resonance (GDD 5.2 <-> 5.6): a land's legends shape its character, so tales
    /// of valor make a martial people and tales of wonder a mystical one. The
    /// myth reinforces the culture its theme embodies.
    pub myth_weight: f32,
    /// Cultural-influence baseline and per-landmark bonus (the reversion target).
    pub influence_base: f32,
    pub influence_per_landmark: f32,
    pub influence_rate: f32,
    /// Per-tick stat aura a landmark radiates into its region, per point of its
    /// influence (GDD 5.2): a scholarly or mystical site deepens the arcane, a
    /// mercantile or pastoral one enriches, a martial one makes the land more
    /// perilous — so a notable place shapes its region's character, not just its
    /// culture.
    pub landmark_aura: f32,
    /// A flourishing, culturally-vibrant region raises a wonder over time (GDD
    /// 5.2): each tick an eligible region rolls `landmark_found_chance`; it must
    /// hold at least `landmark_found_prosperity` prosperity and
    /// `landmark_found_influence_min` cultural influence, and never grows past
    /// `landmark_max_per_region` wonders. A new wonder takes the region's culture
    /// and `landmark_found_influence`.
    pub landmark_found_chance: f32,
    pub landmark_found_prosperity: f32,
    pub landmark_found_influence_min: f32,
    pub landmark_max_per_region: usize,
    pub landmark_found_influence: f32,
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
    /// Fraction each endpoint drifts toward the pair's average magic affinity:
    /// arcana travels the roads too, so a connected, attuned land shares its
    /// arcane current with its partners (GDD 5.2 <-> 5.6). Trade only spreads
    /// magic between regions, never creates it — no flat bonus.
    pub magic_equalize: f32,
    /// How much the more perilous endpoint's danger throttles trade income:
    /// a route is only as safe as its worst leg, so caravans falter where the
    /// road runs through peril (GDD 5.2). Route safety is
    /// `clamp(1 - peril * peril_penalty, min_safety, 1)`.
    pub peril_penalty: f32,
    pub min_safety: f32,
    /// Effective route volume each living Merchant hero at either endpoint adds
    /// (GDD 5.2 <-> 5.4): a merchant plies the road, so a land's caravans carry
    /// more wealth for every trader who calls it home. This gives the Merchant
    /// role real economic weight — the counterpart to how a Warrior lends conquest
    /// might — so hero role now shapes the trade network, not only culture.
    pub merchant_volume_bonus: f32,
    /// Per-tick chance a prospering region forges a new trade route (GDD 5.2):
    /// the trade network was the last part of the world to stay fixed while the
    /// map itself grows — a fractured, conquered, or frontier region was born
    /// economically isolated and never joined the roads. Now wealth reaches for
    /// wealth, so the caravan network grows with the map, the way towns, wonders,
    /// and resource nodes already do.
    pub found_chance: f32,
    /// Both endpoints of a newly forged route must clear this prosperity.
    pub found_min_prosperity: f32,
    /// A region joins at most this many routes, so the network densifies without
    /// every land wiring to every other.
    pub found_max_routes_per_region: usize,
    /// Starting volume of a forged route — thinner than the seeded arteries, a
    /// young road that thickens as its endpoints prosper and merchants ply it.
    pub found_volume: f32,
}
