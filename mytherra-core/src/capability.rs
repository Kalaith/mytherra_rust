//! The §5.9 Standing model: what a player may see, do, and bet on.
//!
//! Standing moves along two orthogonal axes — *visibility* ([`VisibilityScope`])
//! and *influence* ([`ActionVerb`]), plus a [`BettingMarket`] axis for the
//! Observatory. A [`Standing`] is a set of unlocked capabilities; the named
//! [`Tier`] ranks (Watcher → Patron → Shaper → Elder) bundle them **purely
//! additively** — a higher tier grants more and never revokes a lower one's
//! grants. The tier → capability mapping itself is data-driven (see
//! [`crate::data::TierTable`], loaded from `tiers.json`); this module holds only
//! the vocabulary and the level→tier rule.

use crate::data::BetPredicate;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A screen/entity class a player's Standing can reveal. Ordered so a
/// serialized `Standing` is deterministic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum VisibilityScope {
    Heroes,
    Observatory,
    Regions,
    Settlements,
    Resources,
    DivineTools,
    Pantheon,
    Eras,
    /// The full chronicle rather than only the most recent events.
    FullChronicle,
}

/// A divine verb a player's Standing can unlock. Gates the matching
/// `PlayerAction` families (GDD 7.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ActionVerb {
    RegionAction,
    Champion,
    Artifact,
    Magic,
    Myth,
    Weather,
    Agenda,
    Pantheon,
}

/// A family of speculation propositions the Observatory can open to a player.
/// Betting is authorized by market (from the event's predicate) rather than by a
/// verb, so it scales from hero-scoped fledgling bets up to region-collapse
/// wagers (§5.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BettingMarket {
    /// A single hero's fate — death, level, renown.
    HeroFate,
    /// A region or settlement's fortunes — its stats crossing a threshold.
    RegionFortune,
    /// The world as a whole — the age ending, a new region rising.
    WorldTurning,
    /// A region's destruction — crisis or conquest (the Elder's wager, §5.9).
    RegionCollapse,
}

impl BettingMarket {
    /// The market a speculation predicate belongs to (§5.9).
    pub fn of(predicate: BetPredicate) -> Self {
        use BetPredicate as P;
        match predicate {
            P::HeroDies
            | P::HeroLevelAtLeast
            | P::HeroRenownAtLeast
            | P::HeroSurvivesToAge
            | P::HeroChangesRegion => Self::HeroFate,
            P::RegionCrisis | P::RegionConquered => Self::RegionCollapse,
            P::AgeEnds | P::NewRegion => Self::WorldTurning,
            // Every remaining predicate is a region-stat or settlement threshold.
            _ => Self::RegionFortune,
        }
    }
}

/// A player's unlocked capabilities across all three axes (§5.9). Enforced
/// server-side (§7.7); the client mirrors it only to enable/disable affordances.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Standing {
    /// The tier rank this Standing corresponds to (0 = Watcher).
    pub tier: u8,
    pub scopes: BTreeSet<VisibilityScope>,
    pub verbs: BTreeSet<ActionVerb>,
    pub markets: BTreeSet<BettingMarket>,
}

impl Standing {
    pub fn can_see(&self, scope: VisibilityScope) -> bool {
        self.scopes.contains(&scope)
    }

    pub fn can_do(&self, verb: ActionVerb) -> bool {
        self.verbs.contains(&verb)
    }

    pub fn can_bet(&self, market: BettingMarket) -> bool {
        self.markets.contains(&market)
    }
}

/// The four named ranks of divine Standing (§5.9). The pure ladder — the
/// capabilities each rank grants live in [`crate::data::TierTable`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Tier {
    Watcher,
    Patron,
    Shaper,
    Elder,
}

impl Tier {
    pub const ALL: [Tier; 4] = [Tier::Watcher, Tier::Patron, Tier::Shaper, Tier::Elder];

    pub fn rank(self) -> u8 {
        match self {
            Tier::Watcher => 0,
            Tier::Patron => 1,
            Tier::Shaper => 2,
            Tier::Elder => 3,
        }
    }

    /// The tier's display name (a proper design rank, not tuned content).
    pub fn label(self) -> &'static str {
        match self {
            Tier::Watcher => "Watcher",
            Tier::Patron => "Patron",
            Tier::Shaper => "Shaper",
            Tier::Elder => "Elder",
        }
    }

    /// The tier a player of the given standing-level holds, per the data-driven
    /// `unlock_levels` (the level at which each tier above Watcher opens, in
    /// order Patron/Shaper/Elder). Extra thresholds are ignored; missing ones
    /// simply cap the reachable tier.
    pub fn for_level(level: u32, unlock_levels: &[u32]) -> Tier {
        let mut tier = Tier::Watcher;
        for (i, &threshold) in unlock_levels.iter().enumerate() {
            if level >= threshold {
                if let Some(&next) = Tier::ALL.get(i + 1) {
                    tier = next;
                }
            }
        }
        tier
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_climbs_with_level_by_the_unlock_thresholds() {
        let unlock = [2u32, 4, 7]; // Patron@2, Shaper@4, Elder@7
        assert_eq!(Tier::for_level(1, &unlock), Tier::Watcher);
        assert_eq!(Tier::for_level(2, &unlock), Tier::Patron);
        assert_eq!(Tier::for_level(3, &unlock), Tier::Patron);
        assert_eq!(Tier::for_level(4, &unlock), Tier::Shaper);
        assert_eq!(Tier::for_level(6, &unlock), Tier::Shaper);
        assert_eq!(Tier::for_level(7, &unlock), Tier::Elder);
        assert_eq!(Tier::for_level(99, &unlock), Tier::Elder);
    }

    #[test]
    fn predicates_map_to_the_expected_markets() {
        assert_eq!(
            BettingMarket::of(BetPredicate::HeroDies),
            BettingMarket::HeroFate
        );
        assert_eq!(
            BettingMarket::of(BetPredicate::RegionProsperityAtLeast),
            BettingMarket::RegionFortune
        );
        assert_eq!(
            BettingMarket::of(BetPredicate::RegionConquered),
            BettingMarket::RegionCollapse
        );
        assert_eq!(
            BettingMarket::of(BetPredicate::AgeEnds),
            BettingMarket::WorldTurning
        );
    }
}
