//! Delayed world consequences (GDD 5.6): the aftermath steps of an artifact
//! backlash, scheduled to fire some ticks after the shattering so a relic's
//! failure ripples out over time rather than all at once.

use serde::{Deserialize, Serialize};

/// One scheduled effect and how many ticks remain until it fires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelayedConsequence {
    /// Region the effect lands on.
    pub region_id: String,
    /// What caused it (e.g. the shattered relic), for the chronicle line.
    pub source: String,
    /// Ticks remaining until it fires.
    pub delay: i32,
    pub effect: ConsequenceEffect,
}

/// What a delayed consequence does when it fires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsequenceEffect {
    /// Blight the region's largest settlement (a negative prosperity delta).
    SettlementBlight(f32),
    /// A lingering pulse of unrest on the region itself.
    RegionUnrest { chaos: f32, danger: f32 },
    /// Bless the region's largest settlement with a delayed harvest (a positive
    /// prosperity delta) — the bounty that follows fair weather.
    SettlementBloom(f32),
    /// The arcane shockwave of a shattering strips renown from the region's
    /// living heroes (GDD 5.6 <-> 5.4): a catastrophe the heroes failed to avert
    /// dims their legends, and a hero shorn of renown is the frailer for it just
    /// as the aftermath's unrest raises the danger around them. The value is the
    /// renown each living hero of the region loses.
    HeroesShaken(f32),
}

impl ConsequenceEffect {
    /// Whether this pending effect is something to welcome rather than dread, so
    /// the UI can foretell a coming harvest apart from a coming scar (GDD 5.6).
    pub fn is_boon(&self) -> bool {
        matches!(self, ConsequenceEffect::SettlementBloom(_))
    }
}

#[cfg(test)]
mod tests {
    use super::ConsequenceEffect;

    #[test]
    fn only_a_bloom_is_a_boon() {
        assert!(ConsequenceEffect::SettlementBloom(5.0).is_boon());
        assert!(!ConsequenceEffect::SettlementBlight(5.0).is_boon());
        assert!(!ConsequenceEffect::RegionUnrest {
            chaos: 1.0,
            danger: 1.0
        }
        .is_boon());
    }
}
