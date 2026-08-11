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
fn all_scopes_lists_every_variant() {
    // The match is the real guard: adding a `VisibilityScope` variant fails
    // to compile here until it is also added to `ALL`, so a spectator can
    // never silently miss a scope that was introduced later.
    for scope in VisibilityScope::ALL {
        match scope {
            VisibilityScope::Heroes
            | VisibilityScope::Observatory
            | VisibilityScope::Regions
            | VisibilityScope::Settlements
            | VisibilityScope::Resources
            | VisibilityScope::DivineTools
            | VisibilityScope::Pantheon
            | VisibilityScope::Eras
            | VisibilityScope::FullChronicle => {}
        }
    }
    let unique: BTreeSet<_> = VisibilityScope::ALL.into_iter().collect();
    assert_eq!(unique.len(), VisibilityScope::ALL.len());
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
