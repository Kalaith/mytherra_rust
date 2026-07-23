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
mod tests {
    use super::*;

    fn balance() -> MagicBalance {
        crate::data::GameData::load().unwrap().balance.magic
    }

    fn path() -> MagicPath {
        MagicPath::from_seed(&MagicPathSeed {
            id: "p".to_owned(),
            name: "P".to_owned(),
            description: String::new(),
            effect_stat: MagicStat::Prosperity,
            effect_per_tick: 0.3,
        })
    }

    #[test]
    fn thresholds_drive_state() {
        let b = balance();
        let mut p = path();
        p.recompute_state(&b);
        assert_eq!(p.state, MagicState::Dormant);

        p.progress = b.emerging_progress;
        p.evidence = b.emerging_evidence;
        p.recompute_state(&b);
        assert_eq!(p.state, MagicState::Emerging);

        p.progress = b.known_progress;
        p.evidence = b.known_evidence;
        p.recompute_state(&b);
        assert_eq!(p.state, MagicState::Known);
    }

    #[test]
    fn effect_scale_matches_state() {
        let b = balance();
        let mut p = path();
        assert_eq!(p.effect_scale(&b), 0.0);
        p.state = MagicState::Known;
        assert_eq!(p.effect_scale(&b), 1.0);
    }
}
