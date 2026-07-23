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
use mytherra_core::command::{apply, authorize, FeedbackLevel};
use mytherra_protocol::PlayerAction;

impl Game {
    /// Issue an authoritative command. Online, it is sent to the server, which
    /// authorizes and applies it (§7.1, §7.7); the report returns on a later
    /// poll. Under the capture fixture there is no server, so it is authorized
    /// and applied locally instead.
    pub(super) fn submit(&mut self, command: PlayerAction) {
        if let Some(session) = self.online.as_mut() {
            session.submit(&command);
            return;
        }
        if !self.authorized(&command) {
            // The deity's Standing has not unlocked this art yet (GDD 5.9).
            self.notifications
                .warning(self.data.strings.notifications.action_locked.clone());
            return;
        }
        self.apply_player_action(command);
    }

    /// Whether the local deity's Standing permits this command (GDD 7.7) — the
    /// same check the server runs, shared via `mytherra_core::command`.
    fn authorized(&self, command: &PlayerAction) -> bool {
        authorize(&self.standing, &self.world, command)
    }

    /// Apply an authorized command through the shared core apply (GDD 7.1) — the
    /// exact logic the server runs — then surface its feedback as notifications.
    /// Used only by the capture fixture; online, the server does the applying.
    pub(super) fn apply_player_action(&mut self, command: PlayerAction) {
        let report = apply(&mut self.world, &mut self.player, &self.data, &command);
        self.view_dirty = true;
        self.surface_feedback(report);
    }

    /// Turn a command's [`ActionReport`](mytherra_core::command::ActionReport)
    /// feedback into player-facing notifications — whether it was applied locally
    /// (capture) or returned over the wire from the server (online).
    pub(super) fn surface_feedback(&mut self, report: mytherra_core::command::ActionReport) {
        for feedback in report.feedback {
            match feedback.level {
                FeedbackLevel::Success => self.notifications.success(feedback.message),
                FeedbackLevel::Warning => self.notifications.warning(feedback.message),
                FeedbackLevel::Info => self.notifications.info(feedback.message),
            }
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
