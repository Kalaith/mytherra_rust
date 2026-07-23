//! The §5.9 Standing model: what a player may see, do, and bet on.
//!
//! Standing moves along two orthogonal axes — *visibility* ([`VisibilityScope`])
//! and *influence* ([`ActionVerb`]), plus a [`BettingMarket`] axis for the
//! Observatory. A [`Standing`] is a set of unlocked capabilities; the reference
//! [`Tier`] ladder bundles them into the four named ranks (Watcher → Patron →
//! Shaper → Elder), **purely additively** — a higher tier grants more and never
//! revokes a lower tier's grants.

use mytherra_core::data::BetPredicate;
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
/// [`PlayerAction`](crate::PlayerAction) families (§7.7).
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
            P::HeroDies | P::HeroLevelAtLeast | P::HeroRenownAtLeast => Self::HeroFate,
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
    /// The reference tier rank this Standing corresponds to (0 = Watcher).
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

/// The four reference ranks of divine Standing (§5.9). The mapping below is a
/// starting progression baked into the crate; M0.5 will let `tiers.json`
/// override which capabilities land at which rank.
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

    /// The capabilities this tier *adds* on top of the ranks below it (§5.9).
    fn granted(
        self,
    ) -> (
        &'static [VisibilityScope],
        &'static [ActionVerb],
        &'static [BettingMarket],
    ) {
        use ActionVerb as A;
        use BettingMarket as M;
        use VisibilityScope as V;
        match self {
            // A newly-woken deity: sees its heroes and the Observatory, may make
            // the one hero-adjacent nudge of cultivating a champion, and wagers
            // only on a single hero's fate.
            Tier::Watcher => (&[V::Heroes, V::Observatory], &[A::Champion], &[M::HeroFate]),
            // Steward of lands: the classic loop — bless/corrupt/guide regions and
            // wager on their fortunes.
            Tier::Patron => (&[V::Regions], &[A::RegionAction], &[M::RegionFortune]),
            // Shaper of civilization: settlements, resources, and the artifact/
            // magic/myth tools; wagers reach the turning of the age.
            Tier::Shaper => (
                &[V::Settlements, V::Resources, V::DivineTools],
                &[A::Artifact, A::Magic, A::Myth],
                &[M::WorldTurning],
            ),
            // Elder of the pantheon: weather, agendas, and the gods themselves —
            // and the wager that a region will be destroyed (§5.9).
            Tier::Elder => (
                &[V::Pantheon, V::Eras, V::FullChronicle],
                &[A::Weather, A::Agenda, A::Pantheon],
                &[M::RegionCollapse],
            ),
        }
    }

    /// The cumulative [`Standing`] at this tier: additive over every lower rank.
    pub fn standing(self) -> Standing {
        let mut standing = Standing {
            tier: self.rank(),
            ..Standing::default()
        };
        for tier in Tier::ALL {
            if tier.rank() <= self.rank() {
                let (scopes, verbs, markets) = tier.granted();
                standing.scopes.extend(scopes.iter().copied());
                standing.verbs.extend(verbs.iter().copied());
                standing.markets.extend(markets.iter().copied());
            }
        }
        standing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_are_purely_additive() {
        // Each rank's Standing is a superset of the one below it, on every axis.
        for pair in Tier::ALL.windows(2) {
            let (lo, hi) = (pair[0].standing(), pair[1].standing());
            assert!(hi.scopes.is_superset(&lo.scopes), "scopes shrank");
            assert!(hi.verbs.is_superset(&lo.verbs), "verbs shrank");
            assert!(hi.markets.is_superset(&lo.markets), "markets shrank");
        }
    }

    #[test]
    fn a_watcher_sees_heroes_but_not_regions() {
        let watcher = Tier::Watcher.standing();
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
    fn only_the_elder_may_shape_weather_and_wager_on_collapse() {
        let elder = Tier::Elder.standing();
        assert!(elder.can_do(ActionVerb::Weather));
        assert!(elder.can_bet(BettingMarket::RegionCollapse));
        // ...and still retains everything a Patron could do (additive).
        assert!(elder.can_do(ActionVerb::RegionAction));
        assert!(elder.can_see(VisibilityScope::Regions));

        let shaper = Tier::Shaper.standing();
        assert!(!shaper.can_do(ActionVerb::Weather));
        assert!(!shaper.can_bet(BettingMarket::RegionCollapse));
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
