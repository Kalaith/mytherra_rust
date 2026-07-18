//! Runtime region state: the mutable, simulated form of a `RegionSeed`.

use crate::data::{ClimateType, Culture, RegionActionDef, RegionBalance, RegionSeed};
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
    pub fn from_seed(seed: &RegionSeed, balance: &RegionBalance) -> Self {
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
        region.refresh_status(balance);
        region
    }

    /// Cost multiplier from divine resonance: high-resonance regions are cheaper
    /// to nudge (GDD 5.2, tuned in `balance.json`).
    pub fn cost_multiplier(&self, balance: &RegionBalance) -> f32 {
        let curve = &balance.cost_multiplier;
        (1.0 - (self.divine_resonance - 50.0) * curve.coeff).clamp(curve.min, curve.max)
    }

    /// Effect multiplier from divine resonance: high-resonance regions respond
    /// more strongly (GDD 5.2, tuned in `balance.json`).
    pub fn effect_multiplier(&self, balance: &RegionBalance) -> f32 {
        let curve = &balance.effect_multiplier;
        (1.0 + (self.divine_resonance - 50.0) * curve.coeff).clamp(curve.min, curve.max)
    }

    /// Final favor cost of an action against this region.
    pub fn action_cost(&self, def: &RegionActionDef, balance: &RegionBalance) -> i64 {
        ((def.cost as f32 * self.cost_multiplier(balance)).round() as i64).max(1)
    }

    /// Apply an action's resonance-scaled stat deltas. Does not touch favor;
    /// callers debit the player after confirming affordability.
    pub fn apply_action(&mut self, def: &RegionActionDef, balance: &RegionBalance) {
        let mult = self.effect_multiplier(balance);
        self.prosperity = clamp_stat(self.prosperity + scaled(def.prosperity, mult));
        self.chaos = clamp_stat(self.chaos + scaled(def.chaos, mult));
        self.danger = clamp_stat(self.danger + scaled(def.danger, mult));
        self.magic_affinity = clamp_stat(self.magic_affinity + scaled(def.magic_affinity, mult));
        self.refresh_status(balance);
    }

    /// Nudge cultural influence (from myth echoes), clamped 0-100.
    pub fn adjust_culture(&mut self, amount: f32) {
        self.cultural_influence = clamp_stat(self.cultural_influence + amount);
    }

    /// Composite unrest pressure (GDD 5.6 omen formula), reused by champion
    /// rivalry resolution as the region's threat baseline.
    pub fn pressure(&self) -> f32 {
        self.chaos * 0.38 + self.danger * 0.42 + (100.0 - self.prosperity) * 0.2
    }

    /// Apply raw (already-computed) stat deltas, clamp, and refresh status.
    /// Used by systems other than divine actions (champion rivalries, artifacts).
    pub fn apply_deltas(
        &mut self,
        prosperity: f32,
        chaos: f32,
        danger: f32,
        magic: f32,
        balance: &RegionBalance,
    ) {
        self.prosperity = clamp_stat(self.prosperity + prosperity);
        self.chaos = clamp_stat(self.chaos + chaos);
        self.danger = clamp_stat(self.danger + danger);
        self.magic_affinity = clamp_stat(self.magic_affinity + magic);
        self.refresh_status(balance);
    }

    /// Recompute the derived status band from current stats (thresholds from
    /// `balance.json`).
    pub fn refresh_status(&mut self, balance: &RegionBalance) {
        let t = &balance.status;
        self.status = if self.danger >= t.wartorn_danger && self.chaos >= t.wartorn_chaos {
            RegionStatus::WarTorn
        } else if self.chaos >= t.unrest_chaos {
            RegionStatus::Unrest
        } else if self.prosperity >= t.thriving_prosperity && self.chaos < t.thriving_chaos_max {
            RegionStatus::Thriving
        } else if self.prosperity < t.struggling_prosperity {
            RegionStatus::Struggling
        } else if self.prosperity >= t.prospering_prosperity {
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

    fn balance() -> crate::data::Balance {
        crate::data::GameData::load().unwrap().balance
    }

    #[test]
    fn neutral_resonance_gives_base_cost_and_effect() {
        let b = balance();
        let mut region = Region::from_seed(&seed(), &b.region);
        assert_eq!(region.action_cost(&bless(), &b.region), 15);
        region.apply_action(&bless(), &b.region);
        assert!((region.prosperity - 58.0).abs() < f32::EPSILON);
        assert!((region.chaos - 46.0).abs() < f32::EPSILON);
        assert!((region.danger - 47.0).abs() < f32::EPSILON);
    }

    #[test]
    fn high_resonance_is_cheaper_and_stronger() {
        let b = balance();
        let mut s = seed();
        s.divine_resonance = 100.0;
        let region = Region::from_seed(&s, &b.region);
        assert!(region.action_cost(&bless(), &b.region) < 15);
        assert!(region.effect_multiplier(&b.region) > 1.0);
    }

    #[test]
    fn stats_clamp_to_valid_range() {
        let b = balance();
        let mut s = seed();
        s.prosperity = 98.0;
        let mut region = Region::from_seed(&s, &b.region);
        for _ in 0..10 {
            region.apply_action(&bless(), &b.region);
        }
        assert!(region.prosperity <= 100.0);
        assert!(region.danger >= 0.0);
    }
}
