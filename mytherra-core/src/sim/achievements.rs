//! Achievement unlocking: state-based goals tied to the world and the player's
//! standing, evaluated each tick. Definitions live in `achievements.json`; the
//! unlock condition for each id lives here (arbitrary predicates can't be
//! authored in JSON), and unlock state persists in the player's save.
//!
//! This is core, not client, so the authority server evaluates the very same
//! predicates online — a client-side unlock would be clobbered by the server's
//! player copy on the next poll, so the unlock (and its experience grant) must
//! happen server-side (GDD 7.1, 7.7). The capture fixture drives it through the
//! same [`check`].

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
/// already unlocked is never reported again. Each fresh unlock also awards the
/// deity `achievement_experience`, so milestones feed standing progression
/// rather than being vanity — the grant lives here so client and server can't
/// drift.
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
mod tests;
