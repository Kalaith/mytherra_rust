//! Hero lifecycle and migration tuning (GDD 5.4).

use crate::data::hero::HeroRole;
use serde::{Deserialize, Serialize};

/// Hero lifecycle tuning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroBalance {
    pub life_expectancy_base: f32,
    pub life_expectancy_per_level: f32,
    pub level_up: LevelUpCurve,
    pub death: DeathParams,
    pub move_chance: f32,
    pub migration: MigrationBalance,
    pub renown: RenownParams,
}

/// Hero fame tuning (GDD 5.4): how renown accrues, the danger-death it staves
/// off, and the ascending renown thresholds at which each title in
/// `strings.heroes.renown_titles` is earned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenownParams {
    /// Renown gained each time the hero gains a level.
    pub per_level: f32,
    /// Renown gained for surviving an era transition.
    pub per_era: f32,
    /// Danger-death chance shaved off per point of renown — a legend clings to
    /// life against the odds (never below the death floor).
    pub survival_coeff: f32,
    /// Ascending renown needed for each title (index-aligned with the titles).
    pub thresholds: Vec<f32>,
}

/// Hero migration tuning: where a hero that decides to move goes is no longer
/// uniform-random — each role is drawn to different region stats, so warriors
/// flow toward conflict, mages toward magic, scholars toward settled culture,
/// and rangers toward wilder lands. This ties heroes into the region and genesis
/// systems: heroes abandoning a war-torn region leave it undefended (more
/// conquerable), while thriving regions gather the veterans who found frontiers.
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
    pub merchant: StatWeights,
    pub cleric: StatWeights,
}

impl RoleMigrationWeights {
    pub fn get(&self, role: HeroRole) -> &StatWeights {
        match role {
            HeroRole::Warrior => &self.warrior,
            HeroRole::Mage => &self.mage,
            HeroRole::Scholar => &self.scholar,
            HeroRole::Ranger => &self.ranger,
            HeroRole::Merchant => &self.merchant,
            HeroRole::Cleric => &self.cleric,
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
    /// Trial by fire (GDD 5.4): a hero forged in a dangerous land grows faster.
    /// Level-up chance is scaled by `1 + region danger * crucible_coeff`, so a
    /// warrior who flows toward peril is tempered by it.
    pub crucible_coeff: f32,
    /// Only levels that are a multiple of this are worth a chronicle line, so the
    /// Event Log marks a hero's milestones rather than every step of a steady
    /// climb (GDD 10). Heroes still gain every level and its renown silently.
    pub chronicle_interval: u32,
}

/// Per-tick death roll parameters (GDD 5.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeathParams {
    pub elder_roll: f32,
    pub danger_divisor: f32,
    pub level_divisor: f32,
    pub min_chance: f32,
}
