//! Divine Observatory betting and divine-tool tuning (GDD 5.5, 5.6): artifacts,
//! weather, magic, myths, the pantheon, and civilization agendas.

use crate::data::artifact::ArtifactFocus;
use serde::{Deserialize, Serialize};

/// Divine Observatory betting tuning (GDD 5.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BettingBalance {
    /// How many active (unresolved) speculation events to keep available.
    pub active_events: usize,
    /// Hard cap on stored events before old resolved ones are pruned.
    pub event_cap: usize,
    /// Cap on retained *resolved* wagers; pending wagers are never pruned.
    pub bet_history_cap: usize,
    /// Selectable stake amounts.
    pub stake_presets: Vec<i64>,
    /// Range of simulated crowd stake seeded onto each outcome.
    pub crowd_seed_min: f32,
    pub crowd_seed_max: f32,
    /// Bounds on the world-state-derived target odds modifier.
    pub target_mod_min: f32,
    pub target_mod_max: f32,
    /// Bounds on the crowd-lean payout adjustment.
    pub crowd_lean_min: f32,
    pub crowd_lean_max: f32,
    /// Bounds on the final gross payout multiplier.
    pub payout_min_mult: f32,
    pub payout_max_mult: f32,
    /// Floor on final odds.
    pub min_odds: f32,
}

/// Artifact tool tuning (GDD 5.6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactBalance {
    pub max_active: usize,
    pub create_cost: i64,
    pub empower_base_cost: i64,
    pub empower_power_mult: i64,
    pub empower_instability_div: f32,
    pub transfer_cost: i64,
    pub stabilize_cost: i64,
    pub stabilize_amount: f32,
    pub empower_instability_gain: f32,
    pub instability_per_tick: f32,
    pub instability_power_mult: f32,
    pub backlash_threshold: f32,
    pub backlash_chaos: f32,
    pub backlash_danger: f32,
    pub focus_effect: ArtifactFocusEffect,
}

/// Per-power stat magnitude each artifact focus applies to its region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactFocusEffect {
    pub protection: f32,
    pub prosperity: f32,
    pub war: f32,
    pub knowledge: f32,
}

impl ArtifactFocusEffect {
    pub fn per_power(&self, focus: ArtifactFocus) -> f32 {
        match focus {
            ArtifactFocus::Protection => self.protection,
            ArtifactFocus::Prosperity => self.prosperity,
            ArtifactFocus::War => self.war,
            ArtifactFocus::Knowledge => self.knowledge,
        }
    }
}

/// Weather tool tuning (GDD 5.6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherBalance {
    pub base_cost: i64,
    pub decay_per_tick: f32,
    pub min_magnitude: f32,
    pub max_active: usize,
    /// Per-tick chance a natural weather front arises somewhere, biased by the
    /// region's climate — the skies live without the player shaping them.
    pub natural_chance: f32,
    /// Chance a natural front is Strong rather than Gentle.
    pub natural_strong_chance: f32,
    /// Intensity ids a natural front uses (never Cataclysmic — that's the
    /// player's alone).
    pub natural_gentle_id: String,
    pub natural_strong_id: String,
}

/// Pantheon tool tuning (GDD 5.6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PantheonBalance {
    pub appease_cost: i64,
    pub challenge_cost: i64,
    pub appease_amount: f32,
    pub challenge_amount: f32,
    /// How much an action ripples to the target's ally / rival.
    pub ripple: f32,
    pub cooldown: i32,
    pub drift_target: f32,
    pub drift_rate: f32,
    /// How strongly the world's average of a deity's domain stat shifts its
    /// pressure target away from the baseline (GDD 5.6): a deity whose domain is
    /// ascendant across the world stirs on its own, so a dangerous age rouses the
    /// war god and a prosperous one both its patron and its nemesis.
    pub domain_response: f32,
    /// Ascending pressure tier thresholds and their effect multipliers.
    pub tiers: Vec<f32>,
    pub tier_mults: Vec<f32>,
}

/// Civilization tool tuning (GDD 5.6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivilizationBalance {
    pub apply_threshold: f32,
    pub advance_cost: i64,
    pub advance_boost: f32,
    pub boost_decay: f32,
    pub advance_cooldown: i32,
}

/// Myth tool tuning (GDD 5.6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MythBalance {
    pub promote_cost: i64,
    pub cap: usize,
    pub echo_cooldown: i32,
    pub echo_threshold: f32,
    pub candidate_count: usize,
    pub resonance_min: f32,
    pub resonance_max: f32,
    pub resonance_spread: f32,
    pub resonance_scale: f32,
    /// Baseline weight every region carries when a themed myth looks for a home,
    /// so a legend can still arise where its subject is faint — just less often.
    pub region_floor: f32,
}

/// Magic tool tuning (GDD 5.6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagicBalance {
    pub progress_per_tick: f32,
    pub evidence_per_tick: f32,
    pub magic_affinity_coeff: f32,
    pub emerging_progress: f32,
    pub emerging_evidence: f32,
    pub known_progress: f32,
    pub known_evidence: f32,
    pub research_cost: i64,
    pub research_progress_gain: f32,
    pub research_evidence_gain: f32,
    pub emerging_effect_scale: f32,
    pub stat_cap: f32,
}
