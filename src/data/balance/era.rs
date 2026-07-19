//! Era-system tuning (GDD 5.7): the five weighted triggers, transition
//! mechanics, and the per-trigger aftermath left on the reborn world.

use crate::data::era::EraTrigger;
use serde::{Deserialize, Serialize};

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

/// The mark and toll an ending trigger leaves on the transition: stat deltas on
/// the reborn land, plus how deadly the passage is to heroes and how many heirs
/// rise to meet the new age (GDD 5.7). So a Divine War empties the roster while a
/// peaceful-ish end preserves it, and a Collapse rebuilds prosperity but with
/// fewer descendants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AftermathDelta {
    pub prosperity: f32,
    pub chaos: f32,
    pub danger: f32,
    pub magic: f32,
    /// Multiplier on the base hero-death chance during the transition.
    pub death_mult: f32,
    /// Multiplier on the number of descendant heroes that rise.
    pub descendant_mult: f32,
}
