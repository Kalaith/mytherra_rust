//! Runtime magic-research state (GDD 5.6): each path accrues progress and
//! evidence until it becomes emerging, then known, at which point it passively
//! shapes the world.

use crate::data::{MagicBalance, MagicPathSeed, MagicStat};
use serde::{Deserialize, Serialize};

/// How far a research path has matured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MagicState {
    Dormant,
    Emerging,
    Known,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagicPath {
    pub id: String,
    pub name: String,
    pub description: String,
    pub effect_stat: MagicStat,
    pub effect_per_tick: f32,
    pub progress: f32,
    pub evidence: f32,
    pub state: MagicState,
    /// Set once the path first reaches Known, so it is only announced once.
    pub announced_known: bool,
}

impl MagicPath {
    pub fn from_seed(seed: &MagicPathSeed) -> Self {
        Self {
            id: seed.id.clone(),
            name: seed.name.clone(),
            description: seed.description.clone(),
            effect_stat: seed.effect_stat,
            effect_per_tick: seed.effect_per_tick,
            progress: 0.0,
            evidence: 0.0,
            state: MagicState::Dormant,
            announced_known: false,
        }
    }

    /// Recompute the maturity from progress and evidence thresholds (GDD 5.6).
    pub fn recompute_state(&mut self, balance: &MagicBalance) {
        self.state =
            if self.progress >= balance.known_progress && self.evidence >= balance.known_evidence {
                MagicState::Known
            } else if self.progress >= balance.emerging_progress
                && self.evidence >= balance.emerging_evidence
            {
                MagicState::Emerging
            } else {
                MagicState::Dormant
            };
    }

    /// How strongly the path's passive effect applies this tick.
    pub fn effect_scale(&self, balance: &MagicBalance) -> f32 {
        match self.state {
            MagicState::Known => 1.0,
            MagicState::Emerging => balance.emerging_effect_scale,
            MagicState::Dormant => 0.0,
        }
    }
}

#[cfg(test)]
mod tests;
