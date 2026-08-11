//! [`PlayerAction`] — the authoritative commands a client submits.
//!
//! This is the wire form of the client's mutating verbs, with every selector the
//! client keeps locally (the selected region, the chosen bet confidence/stake,
//! the weather pattern/intensity) made explicit so a command carries everything
//! the authority needs to authorize (via [`Standing`](crate::capability::Standing))
//! and apply (via [`super::apply`]). Pure UI intents (screen selection, paging,
//! filter cycling) are *not* here — they never leave the client.

use crate::capability::ActionVerb;
use crate::data::{ArtifactFocus, ChampionFocus};
use serde::{Deserialize, Serialize};

/// A single authoritative command. Targets are addressed by id (stable across
/// the wire), and content choices by index into the relevant `GameData` table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlayerAction {
    /// Bless/Corrupt/Guide a region — `action_id` into `data.region_actions`.
    RegionAction {
        region_id: String,
        action_id: String,
    },
    /// Adopt a living hero as a champion (defaults to Valor focus, as today).
    DesignateChampion {
        hero_id: String,
    },
    /// Pour favor into a bonded champion's cultivation.
    CultivateChampion {
        hero_id: String,
    },
    /// Set a bonded champion's cultivation focus.
    SetChampionFocus {
        hero_id: String,
        focus: ChampionFocus,
    },
    /// Wager on a speculation event with a chosen confidence/stake preset.
    PlaceBet {
        event_id: String,
        confidence_index: usize,
        stake_index: usize,
    },
    /// Forge a new artifact in a region with the given focus.
    CreateArtifact {
        region_id: String,
        focus: ArtifactFocus,
    },
    EmpowerArtifact {
        artifact_id: String,
    },
    StabilizeArtifact {
        artifact_id: String,
    },
    TransferArtifact {
        artifact_id: String,
        to_region_id: String,
    },
    /// Shape weather over a region — pattern/intensity indices into their
    /// `GameData` tables.
    ShapeWeather {
        region_id: String,
        pattern_index: usize,
        intensity_index: usize,
    },
    ResearchMagic {
        path_id: String,
    },
    PromoteMyth {
        candidate_id: String,
    },
    AdvanceAgenda {
        region_id: String,
        agenda_index: usize,
    },
    AppeaseDeity {
        deity_id: String,
    },
    ChallengeDeity {
        deity_id: String,
    },
}

impl PlayerAction {
    /// The verb capability this command requires (§7.7).
    ///
    /// Returns `None` for [`PlayerAction::PlaceBet`], which is authorized by
    /// *market* instead — the target event's predicate decides which
    /// [`BettingMarket`](crate::capability::BettingMarket) is required, so it
    /// can't be known from the command alone.
    pub fn required_verb(&self) -> Option<ActionVerb> {
        use ActionVerb as A;
        Some(match self {
            PlayerAction::RegionAction { .. } => A::RegionAction,
            PlayerAction::DesignateChampion { .. }
            | PlayerAction::CultivateChampion { .. }
            | PlayerAction::SetChampionFocus { .. } => A::Champion,
            PlayerAction::CreateArtifact { .. }
            | PlayerAction::EmpowerArtifact { .. }
            | PlayerAction::StabilizeArtifact { .. }
            | PlayerAction::TransferArtifact { .. } => A::Artifact,
            PlayerAction::ShapeWeather { .. } => A::Weather,
            PlayerAction::ResearchMagic { .. } => A::Magic,
            PlayerAction::PromoteMyth { .. } => A::Myth,
            PlayerAction::AdvanceAgenda { .. } => A::Agenda,
            PlayerAction::AppeaseDeity { .. } | PlayerAction::ChallengeDeity { .. } => A::Pantheon,
            PlayerAction::PlaceBet { .. } => return None,
        })
    }

    /// Whether this command is a wager (authorized by market, not verb).
    pub fn is_bet(&self) -> bool {
        matches!(self, PlayerAction::PlaceBet { .. })
    }
}

#[cfg(test)]
mod tests;
