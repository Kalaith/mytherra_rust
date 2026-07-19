//! Civilization content types: the competing regional agendas (GDD 5.6).

use serde::{Deserialize, Serialize};

/// Which region stat an active agenda nudges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CivStat {
    #[default]
    Prosperity,
    Chaos,
    Danger,
    Magic,
}

/// Which *other* region an outward-facing agenda presses upon (GDD 5.6): a
/// rivalrous civilization resents the most prosperous of its peers, an
/// expansionist one leans on the weakest. `None` keeps the agenda introverted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpilloverTarget {
    #[default]
    None,
    MostProsperous,
    LeastProsperous,
}

/// An authored agenda: how its score is computed from a region (a weighted-
/// linear formula) and what it does to that region while active.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agenda {
    pub id: String,
    pub name: String,
    pub w_prosperity: f32,
    pub w_chaos: f32,
    pub w_danger: f32,
    pub w_magic: f32,
    pub w_culture: f32,
    pub base: f32,
    pub effect_stat: CivStat,
    pub effect_amount: f32,
    /// Outward-facing effect on another region while this agenda dominates (GDD
    /// 5.6). Defaults leave an agenda introverted, so only agendas authored with
    /// a spillover reach beyond their own borders.
    #[serde(default)]
    pub spillover_target: SpilloverTarget,
    #[serde(default)]
    pub spillover_stat: CivStat,
    #[serde(default)]
    pub spillover_amount: f32,
}
