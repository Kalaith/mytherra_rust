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
    /// Prefix pools keyed by the trigger that *ended* the previous age, so a new
    /// age is named after the cataclysm that birthed it (GDD 5.7) — the trigger's
    /// mark endures in the very name, not only in one-time aftermath deltas. An
    /// empty pool (or the first age, which no trigger birthed) falls back to the
    /// generic `prefixes`. `serde(default)` keeps older content loadable.
    #[serde(default)]
    pub trigger_prefixes: TriggerPrefixes,
}

/// Era-name prefix pools, one per era trigger (GDD 5.7). Mirrors the
/// `AftermathDelta` per-trigger table so a new age's name echoes its cause.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TriggerPrefixes {
    #[serde(default)]
    pub cataclysm: Vec<String>,
    #[serde(default)]
    pub collapse: Vec<String>,
    #[serde(default)]
    pub conquest: Vec<String>,
    #[serde(default)]
    pub rupture: Vec<String>,
    #[serde(default)]
    pub divine_war: Vec<String>,
}

impl TriggerPrefixes {
    /// The prefix pool for an age born of this trigger (may be empty).
    pub fn get(&self, trigger: EraTrigger) -> &[String] {
        match trigger {
            EraTrigger::Cataclysm => &self.cataclysm,
            EraTrigger::Collapse => &self.collapse,
            EraTrigger::Conquest => &self.conquest,
            EraTrigger::MagicalRupture => &self.rupture,
            EraTrigger::DivineWar => &self.divine_war,
        }
    }
}
