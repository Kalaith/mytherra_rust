//! Runtime landmark state (GDD 5.2/6): a fixed notable place that pulls its
//! region's culture and raises its cultural influence.

use crate::data::{Culture, LandmarkSeed};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Landmark {
    pub id: String,
    pub name: String,
    pub region_id: String,
    pub culture: Culture,
    pub influence: f32,
}

impl Landmark {
    pub fn from_seed(seed: &LandmarkSeed) -> Self {
        Self {
            id: seed.id.clone(),
            name: seed.name.clone(),
            region_id: seed.region_id.clone(),
            culture: seed.culture,
            influence: seed.influence,
        }
    }
}
