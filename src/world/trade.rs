//! Runtime trade-route state (GDD 5.2): a fixed link between two regions that
//! enriches both, spreads their prosperity, and biases them mercantile.

use crate::data::TradeRouteSeed;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRoute {
    pub id: String,
    pub name: String,
    pub region_a: String,
    pub region_b: String,
    pub volume: f32,
}

impl TradeRoute {
    pub fn from_seed(seed: &TradeRouteSeed) -> Self {
        Self {
            id: seed.id.clone(),
            name: seed.name.clone(),
            region_a: seed.region_a.clone(),
            region_b: seed.region_b.clone(),
            volume: seed.volume,
        }
    }

    pub fn touches(&self, region_id: &str) -> bool {
        self.region_a == region_id || self.region_b == region_id
    }

    /// The other endpoint of a route from a given region, if it touches it.
    pub fn other(&self, region_id: &str) -> Option<&str> {
        if self.region_a == region_id {
            Some(&self.region_b)
        } else if self.region_b == region_id {
            Some(&self.region_a)
        } else {
            None
        }
    }
}
