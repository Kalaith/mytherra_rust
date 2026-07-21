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

/// Dominant regional culture, scored each tick from heroes/landmarks/resources/
/// settlements with an inertia guard (GDD 5.2).
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
    pub const ALL: [Culture; 5] = [
        Culture::Scholarly,
        Culture::Martial,
        Culture::Mystical,
        Culture::Mercantile,
        Culture::Pastoral,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Culture::Scholarly => "Scholarly",
            Culture::Martial => "Martial",
            Culture::Mystical => "Mystical",
            Culture::Mercantile => "Mercantile",
            Culture::Pastoral => "Pastoral",
        }
    }

    /// Stable index into `ALL`, for score accumulation.
    pub fn index(self) -> usize {
        match self {
            Culture::Scholarly => 0,
            Culture::Martial => 1,
            Culture::Mystical => 2,
            Culture::Mercantile => 3,
            Culture::Pastoral => 4,
        }
    }

    /// The resource a land of this character is likeliest to open when its
    /// prospectors strike out (GDD 5.3 <-> 5.2): a mystical land uncovers a
    /// manaspring, a martial one a mine, an agrarian one new farmland — so a new
    /// region grows resources that reinforce the culture it was born with, the
    /// counterpart to how its heroes and myths already reflect its character.
    pub fn favored_resource(self) -> crate::data::ResourceType {
        use crate::data::ResourceType;
        match self {
            Culture::Scholarly => ResourceType::Forest,
            Culture::Martial => ResourceType::Mine,
            Culture::Mystical => ResourceType::Manaspring,
            Culture::Mercantile => ResourceType::Fishery,
            Culture::Pastoral => ResourceType::Farmland,
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
