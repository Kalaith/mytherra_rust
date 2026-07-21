//! Building content types: the building types and the seeded buildings that
//! stand in settlements (GDD 6).

use crate::data::Culture;
use serde::{Deserialize, Serialize};

/// An authored building type (`building_types.json`); its bonus raises the
/// prosperity of the settlement it stands in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingType {
    pub id: String,
    pub name: String,
    pub prosperity_bonus: f32,
    /// The culture this building embodies; a settlement in a region of the same
    /// dominant culture favours raising it (GDD 5.2 <-> 6). `None` fits anywhere.
    #[serde(default)]
    pub culture: Option<Culture>,
    /// Divine resonance this building raises in its region each tick (GDD 6 <->
    /// 5.1): a Temple is a house of worship, so a land studded with them grows
    /// faithful over time — the built counterpart to a Cleric tending faith, and a
    /// third path (beside consecration and clerics) to the favor a hallowed land
    /// tithes. `serde(default)` leaves the secular building types at zero.
    #[serde(default)]
    pub resonance_bonus: f32,
}

/// A seeded building (`buildings.json`) placed in a settlement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingSeed {
    pub id: String,
    pub name: String,
    pub settlement_id: String,
    pub type_id: String,
}
