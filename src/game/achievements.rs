//! Achievement unlocking: state-based goals tied to the world and the player's
//! standing, evaluated each update. Definitions live in `achievements.json`;
//! the unlock condition for each id lives here (arbitrary predicates can't be
//! authored in JSON), and unlock state persists in the player's save.

use crate::data::GameData;
use crate::world::{bet_record, MagicState, PlayerState, WorldState};

/// Whether the achievement `id` has been earned by the current state.
fn earned(id: &str, world: &WorldState, player: &PlayerState, data: &GameData) -> bool {
    let legend_bar = data
        .balance
        .hero
        .renown
        .thresholds
        .last()
        .copied()
        .unwrap_or(f32::INFINITY);
    match id {
        "first_nudge" => player.nudges >= 1,
        "ascendant" => player.level >= 5,
        "divine_hoard" => player.favor >= 500,
        "kingmaker" => player.champions.len() >= data.balance.champion.max_roster,
        "legend_maker" => world.heroes.iter().any(|h| h.renown >= legend_bar),
        "age_witness" => world.era.number >= 2,
        "prophet" => bet_record(&player.bets).won >= 10,
        "meddler" => player.nudges >= 25,
        // A living myth only exists once the player has promoted a candidate.
        "mythwright" => !world.myths.is_empty(),
        "free_spender" => player.favor_spent >= 1000,
        // A metropolis is the top settlement tier: population past the last of the
        // size thresholds.
        "metropolis" => {
            let thresholds = &data.balance.settlement.tier_thresholds;
            world
                .settlements
                .iter()
                .any(|s| s.tier(thresholds) >= thresholds.len())
        }
        "archmage" => world
            .magic_paths
            .iter()
            .any(|p| p.state == MagicState::Known),
        // The map grew: genesis (a fracture or frontier founding) added a region
        // beyond the seeded set.
        "new_lands" => world.regions.len() > data.regions.len(),
        _ => false,
    }
}

/// Unlock every achievement whose condition is now met, returning the display
/// names of those freshly earned (for notification). Idempotent: an achievement
/// already unlocked is never reported again.
pub fn check(world: &WorldState, player: &mut PlayerState, data: &GameData) -> Vec<String> {
    let freshly: Vec<(String, String)> = player
        .achievements
        .iter()
        .filter(|a| !a.unlocked && earned(&a.id, world, player, data))
        .map(|a| (a.id.clone(), a.name.clone()))
        .collect();

    let mut names = Vec::new();
    for (id, name) in freshly {
        if player.achievements.unlock(&id) {
            // A milestone elevates the deity: award experience toward its next
            // standing, so achievements feed progression rather than being vanity.
            player.gain_experience(
                data.balance.player.achievement_experience,
                &data.balance.player,
            );
            names.push(name);
        }
    }
    names
}

#[cfg(test)]
mod tests {
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
}
