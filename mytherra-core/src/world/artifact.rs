//! Runtime artifact state (GDD 5.6): a divine relic bound to a region, with a
//! focus that nudges that region and an instability that rises until stabilized.

use crate::data::{ArtifactBalance, ArtifactFocus, ArtifactSeed};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub name: String,
    pub focus: ArtifactFocus,
    pub power: u32,
    pub instability: f32,
    pub region_id: String,
}

impl Artifact {
    pub fn from_seed(seed: &ArtifactSeed) -> Self {
        Self {
            id: seed.id.clone(),
            name: seed.name.clone(),
            focus: seed.focus,
            power: seed.power.max(1),
            instability: seed.instability.max(0.0),
            region_id: seed.region_id.clone(),
        }
    }

    /// Favor cost to empower: `base + power*mult + instability/div` (GDD 5.6).
    pub fn empower_cost(&self, balance: &ArtifactBalance) -> i64 {
        balance.empower_base_cost
            + self.power as i64 * balance.empower_power_mult
            + (self.instability / balance.empower_instability_div) as i64
    }

    /// The per-tick stat magnitude this artifact applies to its region, scaled
    /// by power (sign carried by the focus effect value).
    pub fn focus_delta(&self, balance: &ArtifactBalance) -> f32 {
        balance.focus_effect.per_power(self.focus) * self.power as f32
    }

    /// How much instability the artifact accrues per tick.
    pub fn instability_growth(&self, region_chaos: f32, balance: &ArtifactBalance) -> f32 {
        balance.instability_per_tick
            + self.power as f32 * balance.instability_power_mult
            + region_chaos.max(0.0) * balance.instability_chaos_coeff
    }
}

#[cfg(test)]
mod tests;
