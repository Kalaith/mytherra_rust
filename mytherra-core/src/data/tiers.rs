//! The data-driven tier → capability mapping (GDD 5.9), loaded from
//! `tiers.json`. Each entry lists the capabilities its [`Tier`] *adds*; the
//! runtime [`Standing`] at a rank folds every entry at or below it (additive).
//!
//! Keeping this in content — not code — lets the progressive-revelation design
//! (which powers land at which tier) be retuned without a recompile, the open
//! question §13.5 flags.

use crate::capability::{ActionVerb, BettingMarket, Standing, Tier, VisibilityScope};
use serde::{Deserialize, Serialize};

/// One tier's additive grant across the three capability axes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierDef {
    pub tier: Tier,
    #[serde(default)]
    pub scopes: Vec<VisibilityScope>,
    #[serde(default)]
    pub verbs: Vec<ActionVerb>,
    #[serde(default)]
    pub markets: Vec<BettingMarket>,
}

/// The full ladder of tier grants, in no required order (folded by rank).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TierTable {
    defs: Vec<TierDef>,
}

impl TierTable {
    /// The cumulative [`Standing`] at `tier`: every tier at or below its rank
    /// folded together, so higher tiers strictly extend lower ones (§5.9).
    pub fn standing(&self, tier: Tier) -> Standing {
        let mut standing = Standing {
            tier: tier.rank(),
            ..Standing::default()
        };
        for def in &self.defs {
            if def.tier.rank() <= tier.rank() {
                standing.scopes.extend(def.scopes.iter().copied());
                standing.verbs.extend(def.verbs.iter().copied());
                standing.markets.extend(def.markets.iter().copied());
            }
        }
        standing
    }

    /// The first named rank the table forgot to define, if any — used to
    /// fail-fast on incomplete `tiers.json`.
    pub fn missing_tier(&self) -> Option<Tier> {
        Tier::ALL
            .into_iter()
            .find(|t| !self.defs.iter().any(|d| d.tier == *t))
    }
}

#[cfg(test)]
mod tests;
