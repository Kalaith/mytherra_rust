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
mod tests {
    use super::*;
    use crate::data::GameData;

    fn tiers() -> TierTable {
        GameData::load().unwrap().tiers
    }

    #[test]
    fn every_named_tier_is_defined() {
        assert_eq!(tiers().missing_tier(), None);
    }

    #[test]
    fn tiers_are_purely_additive() {
        let table = tiers();
        for pair in Tier::ALL.windows(2) {
            let (lo, hi) = (table.standing(pair[0]), table.standing(pair[1]));
            assert!(hi.scopes.is_superset(&lo.scopes), "scopes shrank");
            assert!(hi.verbs.is_superset(&lo.verbs), "verbs shrank");
            assert!(hi.markets.is_superset(&lo.markets), "markets shrank");
        }
    }

    #[test]
    fn a_watcher_sees_heroes_but_not_regions() {
        let watcher = tiers().standing(Tier::Watcher);
        assert!(watcher.can_see(VisibilityScope::Heroes));
        assert!(watcher.can_see(VisibilityScope::Observatory));
        assert!(!watcher.can_see(VisibilityScope::Regions));
        // A Watcher may cultivate a champion (hero-adjacent) but not act on regions.
        assert!(watcher.can_do(ActionVerb::Champion));
        assert!(!watcher.can_do(ActionVerb::RegionAction));
        assert!(watcher.can_bet(BettingMarket::HeroFate));
        assert!(!watcher.can_bet(BettingMarket::RegionCollapse));
    }

    #[test]
    fn only_the_elder_may_shape_weather_and_wager_on_collapse() {
        let table = tiers();
        let elder = table.standing(Tier::Elder);
        assert!(elder.can_do(ActionVerb::Weather));
        assert!(elder.can_bet(BettingMarket::RegionCollapse));
        // ...and still retains everything a Patron could do (additive).
        assert!(elder.can_do(ActionVerb::RegionAction));
        assert!(elder.can_see(VisibilityScope::Regions));

        let shaper = table.standing(Tier::Shaper);
        assert!(!shaper.can_do(ActionVerb::Weather));
        assert!(!shaper.can_bet(BettingMarket::RegionCollapse));
    }
}
