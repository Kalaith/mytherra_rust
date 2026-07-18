//! Hero content types: role and the seeded starting roster.

use serde::{Deserialize, Serialize};

/// A hero's vocation. Four roles at prototype scale (GDD 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeroRole {
    Warrior,
    Mage,
    Scholar,
    Ranger,
}

impl HeroRole {
    pub fn label(self) -> &'static str {
        match self {
            HeroRole::Warrior => "Warrior",
            HeroRole::Mage => "Mage",
            HeroRole::Scholar => "Scholar",
            HeroRole::Ranger => "Ranger",
        }
    }
}

/// A hero's authored starting state (`heroes.json`). `region_id` references a
/// region id from `regions.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroSeed {
    pub id: String,
    pub name: String,
    pub role: HeroRole,
    pub region_id: String,
    pub level: u32,
    pub age: u32,
}
