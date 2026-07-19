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
