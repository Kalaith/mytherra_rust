use super::*;

#[test]
fn a_known_predicate_round_trips() {
    let json = "\"hero_changes_region\"";
    let pred: BetPredicate = serde_json::from_str(json).unwrap();
    assert_eq!(pred, BetPredicate::HeroChangesRegion);
}

#[test]
fn an_unknown_predicate_degrades_instead_of_failing() {
    // A wager kind from a newer server than this build must not fail the whole
    // projection — it deserializes to `Unknown` (forward compatibility).
    let pred: BetPredicate =
        serde_json::from_str("\"some_future_bet_kind\"").expect("unknown predicate must parse");
    assert_eq!(pred, BetPredicate::Unknown);
    assert_eq!(pred.target_kind(), TargetKind::World);
}
