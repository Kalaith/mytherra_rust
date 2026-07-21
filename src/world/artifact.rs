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
mod tests {
    use super::*;

    fn balance() -> ArtifactBalance {
        crate::data::GameData::load().unwrap().balance.artifact
    }

    fn artifact(power: u32, instability: f32) -> Artifact {
        Artifact::from_seed(&ArtifactSeed {
            id: "a".to_owned(),
            name: "A".to_owned(),
            focus: ArtifactFocus::Protection,
            power,
            instability,
            region_id: "r".to_owned(),
        })
    }

    #[test]
    fn empower_cost_grows_with_power_and_instability() {
        let b = balance();
        let low = artifact(1, 0.0).empower_cost(&b);
        assert!(artifact(5, 0.0).empower_cost(&b) > low);
        assert!(artifact(1, 90.0).empower_cost(&b) > low);
    }

    #[test]
    fn protection_focus_reduces_danger() {
        let b = balance();
        assert!(artifact(3, 0.0).focus_delta(&b) < 0.0);
    }

    #[test]
    fn a_transfer_unsettles_a_relic_without_instantly_shattering_it() {
        // Moving a relic must cost real instability (GDD 5.6), but a single move
        // of an already-stable relic must not push it straight past the backlash
        // line — the risk is meant to build, not be an instant kill.
        let b = balance();
        assert!(
            b.transfer_instability > 0.0,
            "a transfer should unsettle the relic"
        );
        assert!(
            b.transfer_instability < b.backlash_threshold,
            "one transfer of a fresh relic must not instantly shatter it"
        );
    }
}
