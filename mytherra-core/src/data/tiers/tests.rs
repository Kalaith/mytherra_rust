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
