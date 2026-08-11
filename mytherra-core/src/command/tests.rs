use super::*;

#[test]
fn a_region_action_spends_favor_and_moves_the_land() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut player = PlayerState::new(&data.config);
    let region_id = world.regions[0].id.clone();
    let action_id = data.ordered_region_actions()[0].id.clone();
    let favor_before = player.favor;

    let report = apply(
        &mut world,
        &mut player,
        &data,
        &PlayerAction::RegionAction {
            region_id,
            action_id,
        },
    );

    assert!(player.favor < favor_before, "the act should cost favor");
    assert!(
        report
            .feedback
            .iter()
            .any(|f| f.level == FeedbackLevel::Success),
        "a successful act reports success"
    );
}

#[test]
fn an_unaffordable_act_warns_and_changes_nothing() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut player = PlayerState::new(&data.config);
    player.favor = 0;
    let region_id = world.regions[0].id.clone();
    let action_id = data.ordered_region_actions()[0].id.clone();
    let artifacts_before = world.artifacts.len();

    let report = apply(
        &mut world,
        &mut player,
        &data,
        &PlayerAction::RegionAction {
            region_id,
            action_id,
        },
    );

    assert_eq!(player.favor, 0);
    assert_eq!(world.artifacts.len(), artifacts_before);
    assert!(report
        .feedback
        .iter()
        .all(|f| f.level == FeedbackLevel::Warning));
}

#[test]
fn the_nudge_cap_bounds_a_regions_influence_per_tick() {
    // GDD 7.5: a region absorbs only so much divine nudging per tick. With
    // favor no object, the *cap* — not affordability — is what eventually
    // turns a nudge away, and a rejected nudge spends nothing and moves
    // nothing. The budget then refills for the next tick.
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut player = PlayerState::new(&data.config);
    player.favor = 1_000_000; // favor is never the limiter in this test
    let bless = PlayerAction::RegionAction {
        region_id: world.regions[0].id.clone(),
        action_id: "bless".to_owned(),
    };

    let mut landed = 0u32;
    loop {
        let favor_before = player.favor;
        let prosperity_before = world.regions[0].prosperity;
        let report = apply(&mut world, &mut player, &data, &bless);
        if report
            .feedback
            .iter()
            .any(|f| f.level == FeedbackLevel::Success)
        {
            landed += 1;
            assert!(player.favor < favor_before, "a landed nudge spends favor");
            assert!(
                landed < 100,
                "the cap should turn nudges away long before this"
            );
            continue;
        }
        // Saturated: a warning, and neither favor nor the land moved.
        assert!(report
            .feedback
            .iter()
            .all(|f| f.level == FeedbackLevel::Warning));
        assert_eq!(player.favor, favor_before, "a capped nudge costs no favor");
        assert_eq!(
            world.regions[0].prosperity, prosperity_before,
            "a capped nudge moves nothing"
        );
        break;
    }
    assert!(landed >= 1, "at least one nudge lands before saturation");

    // A new tick refills the budget, so nudging works again.
    world.regions[0].refresh_nudge_budget();
    let report = apply(&mut world, &mut player, &data, &bless);
    assert!(
        report
            .feedback
            .iter()
            .any(|f| f.level == FeedbackLevel::Success),
        "the per-tick budget refills each tick"
    );
}

#[test]
fn the_nudge_cap_is_shared_across_deities() {
    // GDD 7.5: the cap sums across *every* deity, so a second god cannot act
    // on a region another has already saturated this tick — wealth split
    // between deities dodges nothing.
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut alpha = PlayerState::new(&data.config);
    let mut beta = PlayerState::new(&data.config);
    alpha.favor = 1_000_000;
    beta.favor = 1_000_000;
    let bless = PlayerAction::RegionAction {
        region_id: world.regions[0].id.clone(),
        action_id: "bless".to_owned(),
    };

    // Alpha nudges the region until the shared budget is spent.
    loop {
        let report = apply(&mut world, &mut alpha, &data, &bless);
        if !report
            .feedback
            .iter()
            .any(|f| f.level == FeedbackLevel::Success)
        {
            break;
        }
    }

    // Beta, with full favor and no prior act, is still turned away.
    let beta_favor_before = beta.favor;
    let report = apply(&mut world, &mut beta, &data, &bless);
    assert!(
        report
            .feedback
            .iter()
            .all(|f| f.level == FeedbackLevel::Warning),
        "beta meets the shared per-region cap"
    );
    assert_eq!(
        beta.favor, beta_favor_before,
        "a nudge stopped by the shared cap costs beta nothing"
    );
}
