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
