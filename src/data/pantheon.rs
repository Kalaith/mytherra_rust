//! Pantheon content types: the four AI deities and their domains (GDD 5.6).

use serde::{Deserialize, Serialize};

/// Which region stat a roused deity presses upon the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PantheonStat {
    Prosperity,
    Chaos,
    Danger,
    Magic,
}

impl PantheonStat {
    pub fn label(self) -> &'static str {
        match self {
            PantheonStat::Prosperity => "Prosperity",
            PantheonStat::Chaos => "Chaos",
            PantheonStat::Danger => "Danger",
            PantheonStat::Magic => "Magic",
        }
    }

    /// Whether the world is better off when this stat rises: Prosperity and Magic
    /// are boons, Chaos and Danger banes. Lets the pantheon UI tell the player
    /// whether a rousing deity is worth appeasing or welcoming (GDD 5.6).
    pub fn rising_is_good(self) -> bool {
        matches!(self, PantheonStat::Prosperity | PantheonStat::Magic)
    }
}

/// An authored deity in the fixed ally/rival diamond.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeitySeed {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub ally_id: String,
    pub rival_id: String,
    pub effect_stat: PantheonStat,
    pub effect_amount: f32,
    pub start_pressure: f32,
}
