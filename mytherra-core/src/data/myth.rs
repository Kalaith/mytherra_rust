//! Myth content types: the themes a legend can carry (GDD 5.6).

use super::PantheonStat;
use serde::{Deserialize, Serialize};

/// Which region stat a myth's echo nudges (besides cultural influence).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MythStat {
    Prosperity,
    Chaos,
    Danger,
    Magic,
}

impl From<PantheonStat> for MythStat {
    /// The pantheon and myths share the same four domains, so a god's wrath maps
    /// cleanly onto the tale that remembers it (GDD 5.6 pantheon <-> myths).
    fn from(stat: PantheonStat) -> Self {
        match stat {
            PantheonStat::Prosperity => MythStat::Prosperity,
            PantheonStat::Chaos => MythStat::Chaos,
            PantheonStat::Danger => MythStat::Danger,
            PantheonStat::Magic => MythStat::Magic,
        }
    }
}

/// An authored myth theme: how strongly it spreads culture, its secondary stat
/// effect when the myth echoes, and the regional culture its tale embodies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MythTheme {
    pub id: String,
    pub name: String,
    pub cultural_effect: f32,
    pub stat: MythStat,
    pub stat_effect: f32,
    /// The culture a living myth of this theme reinforces in its home region
    /// (GDD 5.2 <-> 5.6): valor makes a martial people, wonder a mystical one.
    pub culture: super::Culture,
}
