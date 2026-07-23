//! The command seam: the client's half of the client/server boundary (GDD 7.7).
//!
//! Every authoritative verb the player issues is translated into a
//! [`PlayerAction`] (in `apply_action`), then flows through [`Game::submit`]:
//! it is authorized against the local deity's [`Standing`](mytherra_protocol::Standing)
//! and applied to the world. Offline, `apply_player_action` does the applying
//! directly; when the server arrives it will authorize and apply the very same
//! `PlayerAction`, and this path becomes a network submit instead.
//!
//! Pure UI intents (screen/paging/selector cycling) never reach here — they stay
//! in `apply_action`'s match.

use super::Game;
use crate::data::ChampionFocus;
use mytherra_protocol::{BettingMarket, PlayerAction};

impl Game {
    /// Authorize a command against the local deity's Standing, then apply it.
    pub(super) fn submit(&mut self, command: PlayerAction) {
        if !self.authorized(&command) {
            // The deity's Standing has not unlocked this art yet (GDD 5.9). The
            // UI hides most locked affordances, so this guards the rest (e.g. a
            // wager on a market above the player's tier).
            self.notifications
                .warning(self.data.strings.notifications.action_locked.clone());
            return;
        }
        self.apply_player_action(command);
    }

    /// Whether the local deity's Standing permits this command (GDD 7.7).
    fn authorized(&self, command: &PlayerAction) -> bool {
        if let Some(verb) = command.required_verb() {
            return self.standing.can_do(verb);
        }
        // The only verb-less command is a wager, authorized by the market its
        // target event belongs to. An unknown event is left to `place_bet`,
        // which reports it closed.
        if let PlayerAction::PlaceBet { event_id, .. } = command {
            return self
                .world
                .speculations
                .iter()
                .find(|event| &event.id == event_id)
                .map(|event| self.standing.can_bet(BettingMarket::of(event.predicate)))
                .unwrap_or(true);
        }
        true
    }

    /// Apply an authorized command to the world/player by dispatching to the
    /// per-verb handlers. This is the logic the server will own (GDD 7.1).
    fn apply_player_action(&mut self, command: PlayerAction) {
        match command {
            PlayerAction::RegionAction {
                region_id,
                action_id,
            } => self.apply_region_action(&region_id, &action_id),
            PlayerAction::DesignateChampion { hero_id } => self.designate_champion(&hero_id),
            PlayerAction::CultivateChampion { hero_id } => self.cultivate_champion(&hero_id),
            PlayerAction::SetChampionFocus { hero_id, focus } => {
                self.set_champion_focus(&hero_id, focus)
            }
            PlayerAction::PlaceBet {
                event_id,
                confidence_index,
                stake_index,
            } => self.place_bet(&event_id, confidence_index, stake_index),
            PlayerAction::CreateArtifact { region_id, focus } => {
                self.create_artifact(&region_id, focus)
            }
            PlayerAction::EmpowerArtifact { artifact_id } => self.empower_artifact(&artifact_id),
            PlayerAction::StabilizeArtifact { artifact_id } => {
                self.stabilize_artifact(&artifact_id)
            }
            PlayerAction::TransferArtifact {
                artifact_id,
                to_region_id,
            } => self.transfer_artifact(&artifact_id, &to_region_id),
            PlayerAction::ShapeWeather {
                region_id,
                pattern_index,
                intensity_index,
            } => self.shape_weather(&region_id, pattern_index, intensity_index),
            PlayerAction::ResearchMagic { path_id } => self.research_magic(&path_id),
            PlayerAction::PromoteMyth { candidate_id } => self.promote_myth(&candidate_id),
            PlayerAction::AdvanceAgenda {
                region_id,
                agenda_index,
            } => self.advance_agenda(&region_id, agenda_index),
            PlayerAction::AppeaseDeity { deity_id } => self.appease_deity(&deity_id),
            PlayerAction::ChallengeDeity { deity_id } => self.challenge_deity(&deity_id),
        }
    }

    // --- client-side selector → command resolution -------------------------

    /// The id of the currently selected region, clamped to the roster as the
    /// map grows and shrinks (empty string only if the world has no regions).
    pub(super) fn selected_region_id(&self) -> String {
        let index = self
            .selected_region
            .min(self.world.regions.len().saturating_sub(1));
        self.world
            .regions
            .get(index)
            .map(|r| r.id.clone())
            .unwrap_or_default()
    }

    /// The region an artifact would transfer to: the next one round-robin from
    /// its current home. `None` if the artifact is unknown or the map has fewer
    /// than two regions to move between.
    pub(super) fn next_region_for_artifact(&self, artifact_id: &str) -> Option<String> {
        if self.world.regions.len() < 2 {
            return None;
        }
        let current = self
            .world
            .artifacts
            .iter()
            .find(|a| a.id == artifact_id)?
            .region_id
            .clone();
        let cur_idx = self.world.regions.iter().position(|r| r.id == current)?;
        let next = &self.world.regions[(cur_idx + 1) % self.world.regions.len()];
        Some(next.id.clone())
    }

    /// The focus a champion would cycle to next, if the hero is a champion.
    pub(super) fn next_champion_focus(&self, hero_id: &str) -> Option<ChampionFocus> {
        self.player
            .champions
            .iter()
            .find(|c| c.hero_id == hero_id)
            .map(|c| c.focus.next())
    }
}
