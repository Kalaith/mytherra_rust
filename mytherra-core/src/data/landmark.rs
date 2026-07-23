//! Landmark content types: the notable places within regions (GDD 5.2, 6).

use crate::data::Culture;
use serde::{Deserialize, Serialize};

/// A seeded landmark (`landmarks.json`). Its culture pulls its region's dominant
/// culture, and its influence raises the region's cultural influence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandmarkSeed {
    pub id: String,
    pub name: String,
    pub region_id: String,
    pub culture: Culture,
    pub influence: f32,
}

/// Name parts a newly-raised wonder draws from (`landmark_names.json`): a prefix
/// and a noun combine into a grand name like "The Golden Spire" (GDD 5.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandmarkNameBank {
    pub prefixes: Vec<String>,
    pub nouns: Vec<String>,
}
