//! Era content types: the trigger taxonomy and the era-name banks (GDD 5.7).

use serde::{Deserialize, Serialize};

/// The five pressures that can force an era transition; the dominant one names
/// the era's cause (GDD 5.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EraTrigger {
    Cataclysm,
    Collapse,
    Conquest,
    MagicalRupture,
    DivineWar,
}

impl EraTrigger {
    pub fn label(self) -> &'static str {
        match self {
            EraTrigger::Cataclysm => "Cataclysm",
            EraTrigger::Collapse => "Collapse",
            EraTrigger::Conquest => "Conquest",
            EraTrigger::MagicalRupture => "Magical Rupture",
            EraTrigger::DivineWar => "Divine War",
        }
    }
}

/// Word banks for generating era and descendant-hero names (GDD 9 flags the
/// original's banks as thin; this is the expanded pool).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EraNameBank {
    pub prefixes: Vec<String>,
    pub titles: Vec<String>,
    /// Name templates with `{prefix}`/`{title}` slots, so eras aren't all cast
    /// in one rigid mold (e.g. "Age of {title}", "The {title}"). `serde(default)`
    /// falls back to the classic single form when absent.
    #[serde(default)]
    pub patterns: Vec<String>,
    pub descendant_titles: Vec<String>,
}
