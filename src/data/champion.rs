//! Champion content types: the focus a player cultivates a champion toward.

use serde::{Deserialize, Serialize};

/// A champion's cultivation focus, shaping cost and quest speed (GDD 5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChampionFocus {
    Valor,
    Wisdom,
    Devotion,
}

impl ChampionFocus {
    pub fn label(self) -> &'static str {
        match self {
            ChampionFocus::Valor => "Valor",
            ChampionFocus::Wisdom => "Wisdom",
            ChampionFocus::Devotion => "Devotion",
        }
    }

    /// The focus a champion cycles to next (for the UI cycle button).
    pub fn next(self) -> ChampionFocus {
        match self {
            ChampionFocus::Valor => ChampionFocus::Wisdom,
            ChampionFocus::Wisdom => ChampionFocus::Devotion,
            ChampionFocus::Devotion => ChampionFocus::Valor,
        }
    }
}
