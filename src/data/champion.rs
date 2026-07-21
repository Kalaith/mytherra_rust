//! Champion content types: the focus a player cultivates a champion toward.

use crate::data::HeroRole;
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

    /// Whether this focus suits a hero of the given role — a calling that plays to
    /// their nature (GDD 5.4): Valor for the martial (Warrior, Ranger), Wisdom for
    /// the learned (Mage, Scholar), Devotion for the devout and the giving (Cleric,
    /// Merchant). A champion cultivated along their grain shapes their land more
    /// strongly, so role and focus become a choice that rewards matching, not an
    /// arbitrary pick. Every role has exactly one suited focus, so no hero is left
    /// without a synergistic path.
    pub fn suits(self, role: HeroRole) -> bool {
        matches!(
            (self, role),
            (ChampionFocus::Valor, HeroRole::Warrior | HeroRole::Ranger)
                | (ChampionFocus::Wisdom, HeroRole::Mage | HeroRole::Scholar)
                | (
                    ChampionFocus::Devotion,
                    HeroRole::Cleric | HeroRole::Merchant
                )
        )
    }
}
