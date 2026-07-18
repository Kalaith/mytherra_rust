//! Runtime building state (GDD 6): a building standing in a settlement, its
//! prosperity bonus resolved from its type at world creation.

use crate::data::{BuildingSeed, BuildingType};
use macroquad_toolkit::data_loader::DataRegistry;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Building {
    pub id: String,
    pub name: String,
    pub settlement_id: String,
    /// The building type this was built from — a settlement holds at most one of
    /// each type, so construction (GDD 6) dedupes on it.
    pub type_id: String,
    /// Prosperity bonus this building lends its settlement, from its type.
    pub prosperity_bonus: f32,
}

impl Building {
    pub fn from_seed(seed: &BuildingSeed, types: &DataRegistry<BuildingType>) -> Self {
        let prosperity_bonus = types
            .get(&seed.type_id)
            .map(|t| t.prosperity_bonus)
            .unwrap_or(0.0);
        Self {
            id: seed.id.clone(),
            name: seed.name.clone(),
            settlement_id: seed.settlement_id.clone(),
            type_id: seed.type_id.clone(),
            prosperity_bonus,
        }
    }
}
