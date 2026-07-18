//! Region content types: climate, culture, and the seeded starting stats.

use serde::{Deserialize, Serialize};

/// Broad climate band, used for flavor and (later) resource/weather modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClimateType {
    Temperate,
    Arid,
    Tropical,
    Frozen,
    Coastal,
    Highland,
}

impl ClimateType {
    pub fn label(self) -> &'static str {
        match self {
            ClimateType::Temperate => "Temperate",
            ClimateType::Arid => "Arid",
            ClimateType::Tropical => "Tropical",
            ClimateType::Frozen => "Frozen",
            ClimateType::Coastal => "Coastal",
            ClimateType::Highland => "Highland",
        }
    }
}

/// Dominant regional culture. Scored from heroes/landmarks/resources each tick
/// once those systems exist (GDD 5.2); for now it is a fixed seed value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Culture {
    Scholarly,
    Martial,
    Mystical,
    Mercantile,
    Pastoral,
}

impl Culture {
    pub fn label(self) -> &'static str {
        match self {
            Culture::Scholarly => "Scholarly",
            Culture::Martial => "Martial",
            Culture::Mystical => "Mystical",
            Culture::Mercantile => "Mercantile",
            Culture::Pastoral => "Pastoral",
        }
    }
}

/// A region's seeded starting state, as authored in `regions.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionSeed {
    pub id: String,
    pub name: String,
    pub climate: ClimateType,
    pub culture: Culture,
    pub prosperity: f32,
    pub chaos: f32,
    pub danger: f32,
    pub magic_affinity: f32,
    pub population: f32,
    pub cultural_influence: f32,
    pub divine_resonance: f32,
}
