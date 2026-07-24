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
use mytherra_protocol::{PlayerAction, WorldView};

/// The id of the region selected at `selected_region`, clamped to the revealed
/// roster as the map grows and shrinks (empty string only if the view has no
/// regions). A free function over the [`WorldView`] so it can be tested without
/// a full [`Game`].
fn selected_region_id_in(view: &WorldView, selected_region: usize) -> String {
    let index = selected_region.min(view.regions.len().saturating_sub(1));
    view.regions
        .get(index)
        .map(|r| r.id.clone())
        .unwrap_or_default()
}

/// The region an artifact would transfer to: the next one round-robin from its
/// current home. `None` if the artifact is unknown or the view has fewer than
/// two regions to move between. A free function over the [`WorldView`] so it can
/// be tested without a full [`Game`].
fn next_region_for_artifact_in(view: &WorldView, artifact_id: &str) -> Option<String> {
    if view.regions.len() < 2 {
        return None;
    }
    let current = view
        .artifacts
        .iter()
        .find(|a| a.id == artifact_id)?
        .region_id
        .clone();
    let cur_idx = view.regions.iter().position(|r| r.id == current)?;
    let next = &view.regions[(cur_idx + 1) % view.regions.len()];
    Some(next.id.clone())
}

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
        selected_region_id_in(&self.view, self.selected_region)
    }

    /// The region an artifact would transfer to: the next one round-robin from
    /// its current home. `None` if the artifact is unknown or the map has fewer
    /// than two regions to move between.
    pub(super) fn next_region_for_artifact(&self, artifact_id: &str) -> Option<String> {
        next_region_for_artifact_in(&self.view, artifact_id)
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

#[cfg(test)]
mod tests {
    use super::*;
    use mytherra_core::data::{ArtifactFocus, GameData};
    use mytherra_core::world::{Artifact, PlayerState, WorldState};
    use mytherra_protocol::{project, Tier};

    /// A full-visibility view (Elder standing) of a fresh world — the same shape
    /// the online client renders and resolves commands against.
    fn elder_view() -> WorldView {
        let data = GameData::load().unwrap();
        let world = WorldState::new(&data);
        let player = PlayerState::new(&data.config);
        let elder = data.tiers.standing(Tier::Elder);
        project(&world, &player, &elder, &data).0
    }

    #[test]
    fn next_region_for_artifact_resolves_round_robin_from_the_view() {
        let mut view = elder_view();
        assert!(
            view.regions.len() >= 2,
            "test needs at least two revealed regions"
        );
        let first = view.regions[0].id.clone();
        let second = view.regions[1].id.clone();
        // Place an artifact in the first region — the view, not the (online-stale)
        // local world, is where the resolver must find it.
        view.artifacts.push(Artifact {
            id: "test-relic".to_owned(),
            name: "Test Relic".to_owned(),
            focus: ArtifactFocus::Protection,
            power: 1,
            instability: 0.0,
            region_id: first.clone(),
        });

        // Round-robin: an artifact in regions[0] transfers to regions[1].
        assert_eq!(
            next_region_for_artifact_in(&view, "test-relic"),
            Some(second)
        );
        // From the last region it wraps back to the first.
        let last = view.regions.last().unwrap().id.clone();
        let relic = view
            .artifacts
            .iter_mut()
            .find(|a| a.id == "test-relic")
            .unwrap();
        relic.region_id = last;
        assert_eq!(
            next_region_for_artifact_in(&view, "test-relic"),
            Some(first)
        );
        // An unknown artifact resolves to nothing.
        assert_eq!(next_region_for_artifact_in(&view, "no-such"), None);
    }

    #[test]
    fn selected_region_id_reads_and_clamps_to_the_view() {
        let view = elder_view();
        assert_eq!(selected_region_id_in(&view, 0), view.regions[0].id);
        // An out-of-range selection clamps to the last revealed region.
        let last = view.regions.last().unwrap().id.clone();
        assert_eq!(selected_region_id_in(&view, 999), last);
    }
}
