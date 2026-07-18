//! Building content types: the building types and the seeded buildings that
//! stand in settlements (GDD 6).

use serde::{Deserialize, Serialize};

/// An authored building type (`building_types.json`); its bonus raises the
/// prosperity of the settlement it stands in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingType {
    pub id: String,
    pub name: String,
    pub prosperity_bonus: f32,
}

/// A seeded building (`buildings.json`) placed in a settlement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingSeed {
    pub id: String,
    pub name: String,
    pub settlement_id: String,
    pub type_id: String,
}
