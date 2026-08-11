use super::*;

#[test]
fn verbs_map_to_the_expected_capability() {
    assert_eq!(
        PlayerAction::ShapeWeather {
            region_id: "r".into(),
            pattern_index: 0,
            intensity_index: 0
        }
        .required_verb(),
        Some(ActionVerb::Weather)
    );
    assert_eq!(
        PlayerAction::RegionAction {
            region_id: "r".into(),
            action_id: "bless".into()
        }
        .required_verb(),
        Some(ActionVerb::RegionAction)
    );
}

#[test]
fn a_bet_has_no_verb_requirement() {
    let bet = PlayerAction::PlaceBet {
        event_id: "e".into(),
        confidence_index: 0,
        stake_index: 0,
    };
    assert_eq!(bet.required_verb(), None);
    assert!(bet.is_bet());
}

#[test]
fn commands_round_trip_through_json() {
    let action = PlayerAction::CreateArtifact {
        region_id: "aldermoor".into(),
        focus: ArtifactFocus::Protection,
    };
    let json = serde_json::to_string(&action).unwrap();
    let back: PlayerAction = serde_json::from_str(&json).unwrap();
    assert_eq!(action, back);
}
