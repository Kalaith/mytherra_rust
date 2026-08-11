use super::*;

#[test]
fn a_met_condition_unlocks_exactly_once() {
    let data = GameData::load().unwrap();
    let world = WorldState::new(&data);
    let mut player = PlayerState::new(&data.config);
    player
        .achievements
        .sync_definitions(data.achievements.clone());

    // Nothing earned at a fresh start.
    assert!(check(&world, &mut player, &data).is_empty());

    // Meeting "first_nudge" reports it once...
    player.nudges = 1;
    let first = check(&world, &mut player, &data);
    assert!(first.iter().any(|n| n == "First Intervention"));

    // ...and never again, even though the condition still holds.
    let second = check(&world, &mut player, &data);
    assert!(!second.iter().any(|n| n == "First Intervention"));
}

#[test]
fn unlocking_an_achievement_awards_experience() {
    let data = GameData::load().unwrap();
    let world = WorldState::new(&data);
    let mut player = PlayerState::new(&data.config);
    player
        .achievements
        .sync_definitions(data.achievements.clone());
    let xp = data.balance.player.achievement_experience;
    assert!(
        xp > 0,
        "the reward must be a real award to be worth testing"
    );

    let before = player.experience + player.level as i64 * 100_000; // monotone progress proxy
    player.nudges = 1; // earns "first_nudge"
    let unlocked = check(&world, &mut player, &data);
    assert_eq!(unlocked.len(), 1, "exactly one milestone was reached");
    let after = player.experience + player.level as i64 * 100_000;
    assert!(
        after > before,
        "unlocking an achievement should advance the deity's standing"
    );

    // A second check with no fresh unlock awards nothing further.
    let held = player.experience + player.level as i64 * 100_000;
    check(&world, &mut player, &data);
    assert_eq!(
        player.experience + player.level as i64 * 100_000,
        held,
        "no double-award once the achievement is already held"
    );
}

#[test]
fn standing_thresholds_unlock_their_goals() {
    let data = GameData::load().unwrap();
    let world = WorldState::new(&data);
    let mut player = PlayerState::new(&data.config);
    player
        .achievements
        .sync_definitions(data.achievements.clone());

    player.nudges = 25;
    player.favor_spent = 1000;
    let unlocked = check(&world, &mut player, &data);
    assert!(unlocked.iter().any(|n| n == "The Meddler"));
    assert!(unlocked.iter().any(|n| n == "Open-Handed"));
}

#[test]
fn world_milestones_unlock_their_goals() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut player = PlayerState::new(&data.config);
    player
        .achievements
        .sync_definitions(data.achievements.clone());

    // None of the three world milestones hold at a fresh start.
    let fresh = check(&world, &mut player, &data);
    for name in ["The Great City", "Archmage", "New Lands"] {
        assert!(
            !fresh.iter().any(|n| n == name),
            "{name} unlocked too early"
        );
    }

    // A metropolis, a mastered magic school, and a newly-born region.
    let top = data.balance.settlement.tier_thresholds.last().unwrap();
    world.settlements[0].population = top + 10_000.0;
    world.magic_paths[0].state = MagicState::Known;
    let mut newborn = world.regions[0].clone();
    newborn.id = "rift-test".to_owned();
    world.regions.push(newborn);

    let unlocked = check(&world, &mut player, &data);
    assert!(unlocked.iter().any(|n| n == "The Great City"));
    assert!(unlocked.iter().any(|n| n == "Archmage"));
    assert!(unlocked.iter().any(|n| n == "New Lands"));
}
