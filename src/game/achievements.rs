//! Achievement unlocking: state-based goals tied to the world and the player's
//! standing, evaluated each update. Definitions live in `achievements.json`;
//! the unlock condition for each id lives here (arbitrary predicates can't be
//! authored in JSON), and unlock state persists in the player's save.

use crate::data::GameData;
use crate::world::{bet_record, PlayerState, WorldState};

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
}
