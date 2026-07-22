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
    /// How storied the wonder has grown — a multiplier (>= 1.0) on its pull toward
    /// its region's culture, swelling as it stands through the ages (GDD 5.2). It
    /// touches only cultural identity, never the physical aura the structure
    /// radiates. `serde(default)` keeps older saves loadable at full stature.
    #[serde(default = "unit_stature")]
    pub stature: f32,
}

fn unit_stature() -> f32 {
    1.0
}

impl Landmark {
    pub fn from_seed(seed: &LandmarkSeed) -> Self {
        Self {
            id: seed.id.clone(),
            name: seed.name.clone(),
            region_id: seed.region_id.clone(),
            culture: seed.culture,
            influence: seed.influence,
            stature: 1.0,
        }
    }
}
