//! Artifact content types: focus and the seeded starter relics (GDD 5.6).

use serde::{Deserialize, Serialize};

/// What an artifact channels. Each focus nudges a different region stat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactFocus {
    Protection,
    Prosperity,
    War,
    Knowledge,
}

impl ArtifactFocus {
    pub fn label(self) -> &'static str {
        match self {
            ArtifactFocus::Protection => "Protection",
            ArtifactFocus::Prosperity => "Prosperity",
            ArtifactFocus::War => "War",
            ArtifactFocus::Knowledge => "Knowledge",
        }
    }

    /// The focus a newly-created artifact cycles to next (for the UI selector).
    pub fn next(self) -> ArtifactFocus {
        match self {
            ArtifactFocus::Protection => ArtifactFocus::Prosperity,
            ArtifactFocus::Prosperity => ArtifactFocus::War,
            ArtifactFocus::War => ArtifactFocus::Knowledge,
            ArtifactFocus::Knowledge => ArtifactFocus::Protection,
        }
    }
}

/// A seeded starter relic (`artifacts.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactSeed {
    pub id: String,
    pub name: String,
    pub focus: ArtifactFocus,
    pub power: u32,
    pub instability: f32,
    pub region_id: String,
}
