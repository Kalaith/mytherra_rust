//! Simulation and economy tuning, loaded from `balance.json`.
//!
//! Every magic number the world sim and favor economy use lives here rather
//! than in Rust source, per the data-driven design rule. Rust only names the
//! shape; designers tune the values in JSON.

use crate::data::artifact::ArtifactFocus;
use crate::data::champion::ChampionFocus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub region: RegionBalance,
    pub hero: HeroBalance,
    pub champion: ChampionBalance,
    pub betting: BettingBalance,
    pub artifact: ArtifactBalance,
    pub weather: WeatherBalance,
    pub magic: MagicBalance,
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

/// Hero lifecycle tuning (GDD 5.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroBalance {
    pub life_expectancy_base: f32,
    pub life_expectancy_per_level: f32,
    pub level_up: LevelUpCurve,
    pub death: DeathParams,
    pub move_chance: f32,
}

/// Per-tick level-up probability curve: `base * tier_mult * decay^(level-1)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelUpCurve {
    pub base_chance: f32,
    pub low_tier_max_level: u32,
    pub high_tier_min_level: u32,
    pub low_tier_mult: f32,
    pub mid_tier_mult: f32,
    pub high_tier_mult: f32,
    pub decay: f32,
}

/// Per-tick death roll parameters (GDD 5.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeathParams {
    pub elder_roll: f32,
    pub danger_divisor: f32,
    pub level_divisor: f32,
    pub min_chance: f32,
}

/// Champion cultivation, questing, and rivalry tuning (GDD 5.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChampionBalance {
    pub max_roster: usize,
    pub designate_cost: i64,
    pub cultivate_bond_gain: f32,
    pub base_cultivate_cost: i64,
    pub rank_per_bond: f32,
    pub rank_per_quests: f32,
    pub rank_cap: u32,
    pub quest: QuestParams,
    pub rivalry: RivalryParams,
    pub focuses: ChampionFocuses,
}

/// Per-tick quest-progress formula parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestParams {
    pub base: f32,
    pub rank_mult: f32,
    pub bond_div: f32,
    pub level_div: f32,
    pub min: f32,
    pub max: f32,
    pub goal: f32,
}

/// Deterministic rivalry-resolution parameters (strength vs. threat).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RivalryParams {
    pub strength_bond: f32,
    pub strength_rank: f32,
    pub strength_level: f32,
    pub threat_danger: f32,
    pub threat_chaos_div: f32,
    pub resolved_danger: f32,
    pub resolved_chaos: f32,
    pub resolved_prosperity: f32,
    pub escalated_danger: f32,
    pub escalated_chaos: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChampionFocuses {
    pub valor: FocusParams,
    pub wisdom: FocusParams,
    pub devotion: FocusParams,
}

impl ChampionFocuses {
    pub fn get(&self, focus: ChampionFocus) -> &FocusParams {
        match focus {
            ChampionFocus::Valor => &self.valor,
            ChampionFocus::Wisdom => &self.wisdom,
            ChampionFocus::Devotion => &self.devotion,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusParams {
    pub cost_modifier: i64,
    pub quest_bonus: f32,
}

/// Divine Observatory betting tuning (GDD 5.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BettingBalance {
    /// How many active (unresolved) speculation events to keep available.
    pub active_events: usize,
    /// Hard cap on stored events before old resolved ones are pruned.
    pub event_cap: usize,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerBalance {
    pub level_base_cost: i64,
    pub level_cost_step: i64,
}
