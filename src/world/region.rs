//! Runtime region state: the mutable, simulated form of a `RegionSeed`.

use crate::data::{ClimateType, Culture, RegionActionDef, RegionSeed};
use serde::{Deserialize, Serialize};

/// Derived, at-a-glance health of a region. Recomputed from stats each tick
/// rather than stored authoritatively (GDD 5.2 crisis/thriving detection).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionStatus {
    Thriving,
    Prospering,
    Peaceful,
    Unrest,
    Struggling,
    WarTorn,
}

impl RegionStatus {
    pub fn label(self) -> &'static str {
        match self {
            RegionStatus::Thriving => "Thriving",
            RegionStatus::Prospering => "Prospering",
            RegionStatus::Peaceful => "Peaceful",
            RegionStatus::Unrest => "Unrest",
            RegionStatus::Struggling => "Struggling",
            RegionStatus::WarTorn => "War-torn",
        }
    }

    pub fn is_crisis(self) -> bool {
        matches!(self, RegionStatus::WarTorn | RegionStatus::Struggling)
    }
}

/// A 0-100 clamped world stat helper.
fn clamp_stat(value: f32) -> f32 {
    value.clamp(0.0, 100.0)
}

/// The live, simulated state of one region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub id: String,
    pub name: String,
    pub climate: ClimateType,
    pub culture: Culture,
    pub prosperity: f32,
    pub chaos: f32,
    pub danger: f32,
    pub magic_affinity: f32,
    pub population: f32,
    pub cultural_influence: f32,
    pub divine_resonance: f32,
    pub status: RegionStatus,
}

impl Region {
    pub fn from_seed(seed: &RegionSeed) -> Self {
        let mut region = Self {
            id: seed.id.clone(),
            name: seed.name.clone(),
            climate: seed.climate,
            culture: seed.culture,
            prosperity: clamp_stat(seed.prosperity),
            chaos: clamp_stat(seed.chaos),
            danger: clamp_stat(seed.danger),
            magic_affinity: clamp_stat(seed.magic_affinity),
            population: seed.population.max(0.0),
            cultural_influence: clamp_stat(seed.cultural_influence),
            divine_resonance: clamp_stat(seed.divine_resonance),
            status: RegionStatus::Peaceful,
        };
        region.refresh_status();
        region
    }

    /// Cost multiplier from divine resonance: high-resonance regions are cheaper
    /// to nudge. `clamp(0.7, 1.3, 1 - (resonance-50) * 0.006)` (GDD 5.2).
    pub fn cost_multiplier(&self) -> f32 {
        (1.0 - (self.divine_resonance - 50.0) * 0.006).clamp(0.7, 1.3)
    }

    /// Effect multiplier from divine resonance: high-resonance regions respond
    /// more strongly. `clamp(0.75, 1.35, 1 + (resonance-50) * 0.007)` (GDD 5.2).
    pub fn effect_multiplier(&self) -> f32 {
        (1.0 + (self.divine_resonance - 50.0) * 0.007).clamp(0.75, 1.35)
    }

    /// Final favor cost of an action against this region.
    pub fn action_cost(&self, def: &RegionActionDef) -> i64 {
        ((def.cost as f32 * self.cost_multiplier()).round() as i64).max(1)
    }

    /// Apply an action's resonance-scaled stat deltas. Does not touch favor;
    /// callers debit the player after confirming affordability.
    pub fn apply_action(&mut self, def: &RegionActionDef) {
        let mult = self.effect_multiplier();
        self.prosperity = clamp_stat(self.prosperity + scaled(def.prosperity, mult));
        self.chaos = clamp_stat(self.chaos + scaled(def.chaos, mult));
        self.danger = clamp_stat(self.danger + scaled(def.danger, mult));
        self.magic_affinity = clamp_stat(self.magic_affinity + scaled(def.magic_affinity, mult));
        self.refresh_status();
    }

    /// Recompute the derived status band from current stats.
    pub fn refresh_status(&mut self) {
        self.status = if self.danger >= 65.0 && self.chaos >= 60.0 {
            RegionStatus::WarTorn
        } else if self.chaos >= 70.0 {
            RegionStatus::Unrest
        } else if self.prosperity >= 75.0 && self.chaos < 40.0 {
            RegionStatus::Thriving
        } else if self.prosperity < 30.0 {
            RegionStatus::Struggling
        } else if self.prosperity >= 55.0 {
            RegionStatus::Prospering
        } else {
            RegionStatus::Peaceful
        };
    }
}

/// Scale a signed stat delta by an effect multiplier, preserving sign and
/// keeping a minimum magnitude of 1 for non-zero deltas (GDD 5.2).
fn scaled(delta: f32, mult: f32) -> f32 {
    if delta == 0.0 {
        return 0.0;
    }
    let magnitude = (delta.abs() * mult).round().max(1.0);
    magnitude.copysign(delta)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed() -> RegionSeed {
        RegionSeed {
            id: "t".to_owned(),
            name: "Test".to_owned(),
            climate: ClimateType::Temperate,
            culture: Culture::Scholarly,
            prosperity: 50.0,
            chaos: 50.0,
            danger: 50.0,
            magic_affinity: 50.0,
            population: 1000.0,
            cultural_influence: 50.0,
            divine_resonance: 50.0,
        }
    }

    fn bless() -> RegionActionDef {
        RegionActionDef {
            id: "bless".to_owned(),
            name: "Bless".to_owned(),
            description: String::new(),
            cost: 15,
            prosperity: 8.0,
            chaos: -4.0,
            danger: -3.0,
            magic_affinity: 0.0,
        }
    }

    #[test]
    fn neutral_resonance_gives_base_cost_and_effect() {
        let mut region = Region::from_seed(&seed());
        assert_eq!(region.action_cost(&bless()), 15);
        region.apply_action(&bless());
        assert!((region.prosperity - 58.0).abs() < f32::EPSILON);
        assert!((region.chaos - 46.0).abs() < f32::EPSILON);
        assert!((region.danger - 47.0).abs() < f32::EPSILON);
    }

    #[test]
    fn high_resonance_is_cheaper_and_stronger() {
        let mut s = seed();
        s.divine_resonance = 100.0;
        let region = Region::from_seed(&s);
        assert!(region.action_cost(&bless()) < 15);
        assert!(region.effect_multiplier() > 1.0);
    }

    #[test]
    fn stats_clamp_to_valid_range() {
        let mut s = seed();
        s.prosperity = 98.0;
        let mut region = Region::from_seed(&s);
        for _ in 0..10 {
            region.apply_action(&bless());
        }
        assert!(region.prosperity <= 100.0);
        assert!(region.danger >= 0.0);
    }
}
