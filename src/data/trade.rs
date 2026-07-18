//! Trade-route content types: the links between regions (GDD 5.2).

use serde::{Deserialize, Serialize};

/// A seeded trade route (`trade_routes.json`) connecting two regions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRouteSeed {
    pub id: String,
    pub name: String,
    pub region_a: String,
    pub region_b: String,
    pub volume: f32,
}
