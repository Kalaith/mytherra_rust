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
}

/// A seeded building (`buildings.json`) placed in a settlement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingSeed {
    pub id: String,
    pub name: String,
    pub settlement_id: String,
    pub type_id: String,
}
