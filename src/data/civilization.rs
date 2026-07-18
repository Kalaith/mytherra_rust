//! Civilization content types: the competing regional agendas (GDD 5.6).

use serde::{Deserialize, Serialize};

/// Which region stat an active agenda nudges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CivStat {
    Prosperity,
    Chaos,
    Danger,
    Magic,
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
}
