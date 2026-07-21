//! Runtime building state (GDD 6): a building standing in a settlement, its
//! prosperity bonus resolved from its type at world creation.

use crate::data::{BuildingSeed, BuildingType, Culture};
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
    /// The culture this building embodies (resolved from its type at creation),
    /// so it reinforces its region's identity — a Forge speaks for the martial,
    /// a Temple the mystical (GDD 6 <-> 5.2). `serde(default)` keeps older saves
    /// loadable; `None` fits any culture.
    #[serde(default)]
    pub culture: Option<Culture>,
    /// Divine resonance this building raises in its region each tick, resolved
    /// from its type at creation (GDD 6 <-> 5.1): a Temple hallows the land around
    /// it. `serde(default)` keeps older saves loadable.
    #[serde(default)]
    pub resonance_bonus: f32,
}

impl Building {
    pub fn from_seed(seed: &BuildingSeed, types: &DataRegistry<BuildingType>) -> Self {
        let ty = types.get(&seed.type_id);
        Self {
            id: seed.id.clone(),
            name: seed.name.clone(),
            settlement_id: seed.settlement_id.clone(),
            type_id: seed.type_id.clone(),
            prosperity_bonus: ty.map(|t| t.prosperity_bonus).unwrap_or(0.0),
            culture: ty.and_then(|t| t.culture),
            resonance_bonus: ty.map(|t| t.resonance_bonus).unwrap_or(0.0),
        }
    }
}
