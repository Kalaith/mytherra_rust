//! Simulation and economy tuning, loaded from `balance.json`.
//!
//! Every magic number the world sim and favor economy use lives here rather
//! than in Rust source, per the data-driven design rule. Rust only names the
//! shape; designers tune the values in JSON.

use crate::data::artifact::ArtifactFocus;
use crate::data::champion::ChampionFocus;
use crate::data::era::EraTrigger;
use crate::data::resource::ResourceStatus;
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
    pub myth: MythBalance,
    pub civilization: CivilizationBalance,
    pub pantheon: PantheonBalance,
    pub era: EraBalance,
    pub settlement: SettlementBalance,
    pub resource: ResourceBalance,
    pub culture: CultureBalance,
    pub trade: TradeBalance,
    pub player: PlayerBalance,
    pub settings: SettingsBalance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionBalance {
    pub cost_multiplier: MultiplierCurve,
    pub effect_multiplier: MultiplierCurve,
    pub status: StatusThresholds,
    pub drift: DriftParams,
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
    pub magic_target: f32,
    /// Proportional pull toward `magic_target`, so magic — pushed up by
    /// knowledge artifacts / divination / the Growth deity — settles rather than
    /// pinning at the ceiling (mirrors the prosperity mean-reversion).
    pub magic_reversion_rate: f32,
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
    /// Signature stat deltas a champion of this focus adds when it *resolves* a
    /// rivalry, so the focus shapes what kind of impact the champion has (GDD
    /// 5.4), not just how fast it quests.
    pub resolve_prosperity: f32,
    pub resolve_danger: f32,
    pub resolve_magic: f32,
}

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
}

/// Resource-node tuning (GDD 5.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceBalance {
    pub stress_chaos: f32,
    pub stress_danger: f32,
    pub degrade_base: f32,
    pub degrade_stress: f32,
    pub recover_base: f32,
    pub improve_base: f32,
    pub contest_chaos_threshold: f32,
    pub corrupt_base: f32,
    pub corrupt_danger: f32,
    pub region_output_scale: f32,
    pub outputs: ResourceOutputs,
}

/// Output multiplier per resource status (GDD 5.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceOutputs {
    pub active: f32,
    pub blessed: f32,
    pub flourishing: f32,
    pub overworked: f32,
    pub contested: f32,
    pub corrupted: f32,
    pub unstable: f32,
    pub depleted: f32,
}

impl ResourceOutputs {
    pub fn get(&self, status: ResourceStatus) -> f32 {
        match status {
            ResourceStatus::Active => self.active,
            ResourceStatus::Blessed => self.blessed,
            ResourceStatus::Flourishing => self.flourishing,
            ResourceStatus::Overworked => self.overworked,
            ResourceStatus::Contested => self.contested,
            ResourceStatus::Corrupted => self.corrupted,
            ResourceStatus::Unstable => self.unstable,
            ResourceStatus::Depleted => self.depleted,
        }
    }
}

/// Settlement growth tuning (GDD 5.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementBalance {
    pub base_growth: f32,
    pub self_prosperity_div: f32,
    pub region_prosperity_div: f32,
    pub region_chaos_div: f32,
    pub growth_min: f32,
    pub growth_max: f32,
    pub prosperity_drift_rate: f32,
    pub region_contribution: f32,
    /// A settlement builds a new building only once its prosperity and
    /// population clear these floors (GDD 6 — buildings grow with settlements).
    pub construction_prosperity_min: f32,
    pub construction_population_min: f32,
    /// Per-tick chance an eligible settlement raises one new building.
    pub construction_chance: f32,
}

/// Era system tuning (GDD 5.7).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EraBalance {
    pub era_length: u32,
    pub breaking_threshold: f32,
    pub cataclysm_danger: f32,
    pub cataclysm_chaos: f32,
    pub cataclysm_crisis: f32,
    pub collapse_prosperity: f32,
    pub collapse_struggling: f32,
    pub conquest_danger: f32,
    pub conquest_wartorn: f32,
    pub rupture_magic: f32,
    pub rupture_known: f32,
    pub divinewar_stake: f32,
    pub divinewar_fallen: f32,
    pub divinewar_lowfavor: f32,
    pub reincarnate_age_min: u32,
    pub reincarnate_age_max: u32,
    pub death_chance: f32,
    pub death_age: u32,
    pub hero_level_scale: f32,
    pub descendant_min: u32,
    pub descendant_max: u32,
    pub renewal_chaos: f32,
    pub renewal_danger: f32,
    pub renewal_prosperity: f32,
    /// Stat marks the *ending* trigger leaves on the reborn world, layered onto
    /// the base renewal so each age's aftermath reflects how it fell (GDD 5.7).
    pub aftermath: EraAftermath,
}

/// Per-trigger transition aftermath (GDD 5.7).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EraAftermath {
    pub cataclysm: AftermathDelta,
    pub collapse: AftermathDelta,
    pub conquest: AftermathDelta,
    pub rupture: AftermathDelta,
    pub divine_war: AftermathDelta,
}

impl EraAftermath {
    pub fn get(&self, trigger: EraTrigger) -> &AftermathDelta {
        match trigger {
            EraTrigger::Cataclysm => &self.cataclysm,
            EraTrigger::Collapse => &self.collapse,
            EraTrigger::Conquest => &self.conquest,
            EraTrigger::MagicalRupture => &self.rupture,
            EraTrigger::DivineWar => &self.divine_war,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AftermathDelta {
    pub prosperity: f32,
    pub chaos: f32,
    pub danger: f32,
    pub magic: f32,
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

/// Settings-screen tuning (GDD 10): the selectable auto-tick cadences, in real
/// seconds between world ticks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsBalance {
    pub tick_speed_presets: Vec<f32>,
}
