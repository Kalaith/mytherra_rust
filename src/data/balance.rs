//! Simulation and economy tuning, loaded from `balance.json`.
//!
//! Every magic number the world sim and favor economy use lives here rather
//! than in Rust source, per the data-driven design rule. Rust only names the
//! shape; designers tune the values in JSON.

use crate::data::artifact::ArtifactFocus;
use crate::data::champion::ChampionFocus;
use crate::data::era::EraTrigger;
use crate::data::hero::HeroRole;
use crate::data::resource::ResourceStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub region: RegionBalance,
    pub genesis: GenesisBalance,
    pub conquest: ConquestBalance,
    pub frontier: FrontierBalance,
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

/// Region-fracture tuning (GDD 5.2): when a region is torn by sustained chaos
/// and danger, secession pressure ("strife") builds until a hero leads part of
/// it to break away as a wholly new region. See `sim/genesis.rs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisBalance {
    /// A region only accrues strife while its `pressure()` exceeds this.
    pub strife_pressure_threshold: f32,
    /// Base strife gained per tick while over the threshold.
    pub strife_gain: f32,
    /// Extra strife per point of pressure above the threshold.
    pub strife_over_scale: f32,
    /// Strife shed per tick while calm — larger than the gain, so only
    /// *sustained* turmoil fractures a region.
    pub strife_decay: f32,
    /// Upper bound on accumulated strife.
    pub strife_cap: f32,
    /// Strife at which the region fractures (given a founder and the population).
    pub fracture_threshold: f32,
    /// The parent must hold at least this population to split.
    pub min_population: f32,
    /// A living hero of at least this level in the region leads the breakaway;
    /// with no such catalyst, strife keeps building but no region is born.
    pub founder_min_level: u32,
    /// Fraction of the parent's population that leaves with the breakaway.
    pub population_split: f32,
    /// Per-settlement chance that a town in the parent defects to the breakaway.
    pub settlement_defect_chance: f32,
    /// Breakaway starting chaos — it is born in revolt.
    pub child_chaos: f32,
    /// Breakaway starting prosperity — a frontier starts poor.
    pub child_prosperity: f32,
    /// Fraction of the parent's danger the breakaway carries over.
    pub child_danger_carry: f32,
    /// Breakaway starting divine resonance and cultural influence.
    pub child_resonance: f32,
    pub child_cultural_influence: f32,
    /// Relief the parent feels once the pressure vents into a new region.
    pub parent_chaos_relief: f32,
    pub parent_danger_relief: f32,
    pub parent_prosperity_hit: f32,
    /// Secession momentum each fracture adds to the world (feeds Collapse-era
    /// pressure, GDD 5.7), and the ceiling that momentum can reach.
    pub momentum_gain: f32,
    pub momentum_cap: f32,
}

/// Region-conquest tuning (GDD 5.2): a strong region can annex a trade-linked
/// neighbour that has collapsed into crisis, merging the loser into the winner.
/// The inverse of a fracture — it removes a region rather than adding one. See
/// `sim/genesis.rs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConquestBalance {
    /// Military-might weights: a region projects force from its wealth, numbers,
    /// standing threat, and (for martial cultures) a warlike bonus.
    pub might_prosperity: f32,
    pub might_population: f32,
    pub might_danger: f32,
    pub might_martial_bonus: f32,
    /// A region must reach this might to move on a neighbour at all.
    pub aggressor_min_might: f32,
    /// The aggressor's might must exceed the target's by this margin.
    pub conquest_margin: f32,
    /// A living hero of at least this level shields its region from conquest —
    /// the same calibre of hero who would instead lead it to secede.
    pub defender_min_level: u32,
    /// If true, conquest only follows an existing trade route between the pair.
    pub require_trade_link: bool,
    /// Fraction of the loser's population the winner absorbs (the rest is lost).
    pub population_transfer: f32,
    /// Stat marks the war of conquest leaves on the victor.
    pub winner_prosperity: f32,
    pub winner_chaos: f32,
    pub winner_danger: f32,
    /// The world will never be conquered below this many regions.
    pub min_regions: usize,
    /// Conquest momentum each annexation adds to the world (feeds Conquest-era
    /// pressure, GDD 5.7), and the ceiling that momentum can reach.
    pub momentum_gain: f32,
    pub momentum_cap: f32,
}

/// Frontier-founding tuning (GDD 5.2): the third genesis path and the mirror of
/// a fracture — born of prosperity, not strife. A veteran hero in a *thriving*,
/// populous region can lead settlers out to found a new frontier region. See
/// `sim/genesis/frontier.rs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontierBalance {
    /// A hero needs at least this level to lead a founding expedition.
    pub founder_min_level: u32,
    /// The home region must hold at least this population to spare settlers.
    pub parent_min_population: f32,
    /// Per-eligible-hero, per-tick chance of founding — kept low so expansion is
    /// occasional rather than explosive.
    pub found_chance: f32,
    /// Fraction of the home region's population that leaves to settle.
    pub settler_fraction: f32,
    /// The world will never grow past this many regions by founding.
    pub max_regions: usize,
    /// A new frontier's starting stats — a hopeful but raw and wild colony.
    pub child_prosperity: f32,
    pub child_chaos: f32,
    pub child_danger: f32,
    /// Fraction of the home region's magic affinity the frontier inherits.
    pub child_magic_carry: f32,
    pub child_resonance: f32,
    pub child_cultural_influence: f32,
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
    pub migration: MigrationBalance,
}

/// Hero migration tuning (GDD 5.4): where a hero that decides to move goes is no
/// longer uniform-random — each role is drawn to different region stats, so
/// warriors flow toward conflict, mages toward magic, scholars toward settled
/// culture, and rangers toward wilder lands. This ties heroes into the region
/// and genesis systems: heroes abandoning a war-torn region leave it undefended
/// (more conquerable), while thriving regions gather the veterans who found
/// frontiers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationBalance {
    /// Baseline pull every region has before stat weighting.
    pub base_weight: f32,
    /// Floor on a region's computed pull, so it is never zero or negative.
    pub min_weight: f32,
    pub roles: RoleMigrationWeights,
}

/// Per-role stat weighting for migration attractiveness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleMigrationWeights {
    pub warrior: StatWeights,
    pub mage: StatWeights,
    pub scholar: StatWeights,
    pub ranger: StatWeights,
}

impl RoleMigrationWeights {
    pub fn get(&self, role: HeroRole) -> &StatWeights {
        match role {
            HeroRole::Warrior => &self.warrior,
            HeroRole::Mage => &self.mage,
            HeroRole::Scholar => &self.scholar,
            HeroRole::Ranger => &self.ranger,
        }
    }
}

/// How strongly each region stat draws (positive) or repels (negative) a hero.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatWeights {
    pub prosperity: f32,
    pub danger: f32,
    pub magic: f32,
    pub culture: f32,
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
    /// Weight on the world's decaying secession-momentum tally, so regions
    /// fracturing from within (not just low prosperity) drive Collapse pressure.
    pub collapse_momentum_weight: f32,
    /// Secession momentum bled off each tick.
    pub collapse_momentum_decay: f32,
    pub conquest_danger: f32,
    pub conquest_wartorn: f32,
    /// Weight on the world's decaying conquest-momentum tally, so actual region
    /// annexations (not just ambient danger) drive Conquest-era pressure.
    pub conquest_momentum_weight: f32,
    /// Conquest momentum bled off each tick.
    pub conquest_momentum_decay: f32,
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
