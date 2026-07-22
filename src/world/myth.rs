//! Runtime myth state (GDD 5.6): candidates scored from the world that the
//! player can promote into living myths, which periodically echo across their
//! region.

use crate::data::{Culture, MythStat};
use serde::{Deserialize, Serialize};

/// A tale stirring in a region, awaiting promotion into a living myth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MythCandidate {
    pub id: String,
    pub title: String,
    pub theme_name: String,
    pub stat: MythStat,
    pub cultural_effect: f32,
    pub stat_effect: f32,
    /// The regional culture this tale embodies (GDD 5.2 <-> 5.6), denormalized
    /// from its theme.
    pub culture: Culture,
    pub region_id: String,
    pub region_name: String,
    pub resonance: f32,
}

/// A promoted myth that endures and echoes on a cooldown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Myth {
    pub id: String,
    pub title: String,
    pub theme_name: String,
    pub stat: MythStat,
    pub cultural_effect: f32,
    pub stat_effect: f32,
    /// The regional culture this myth reinforces as it endures (GDD 5.2 <-> 5.6).
    pub culture: Culture,
    pub region_id: String,
    pub region_name: String,
    pub resonance: f32,
    /// Years until this myth may echo again.
    pub echo_cooldown: i32,
}

impl Myth {
    pub fn from_candidate(candidate: &MythCandidate, cooldown: i32) -> Self {
        Self {
            id: candidate.id.clone(),
            title: candidate.title.clone(),
            theme_name: candidate.theme_name.clone(),
            stat: candidate.stat,
            cultural_effect: candidate.cultural_effect,
            stat_effect: candidate.stat_effect,
            culture: candidate.culture,
            region_id: candidate.region_id.clone(),
            region_name: candidate.region_name.clone(),
            resonance: candidate.resonance,
            echo_cooldown: cooldown,
        }
    }

    /// Whether the myth is ready and strong enough to echo (GDD 5.6).
    pub fn can_echo(&self, threshold: f32) -> bool {
        self.echo_cooldown <= 0 && self.resonance >= threshold
    }
}
