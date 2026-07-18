//! Settlement content types: the seeded towns within regions (GDD 5.3).

use serde::{Deserialize, Serialize};

/// A seeded settlement (`settlements.json`); `region_id` references a region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementSeed {
    pub id: String,
    pub name: String,
    pub region_id: String,
    pub population: f32,
    pub prosperity: f32,
}
