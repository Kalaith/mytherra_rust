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
    Merchant,
    Cleric,
}

impl HeroRole {
    /// Every role, in declaration order — used when a new hero's role is rolled.
    pub const ALL: [HeroRole; 6] = [
        HeroRole::Warrior,
        HeroRole::Mage,
        HeroRole::Scholar,
        HeroRole::Ranger,
        HeroRole::Merchant,
        HeroRole::Cleric,
    ];

    pub fn label(self) -> &'static str {
        match self {
            HeroRole::Warrior => "Warrior",
            HeroRole::Mage => "Mage",
            HeroRole::Scholar => "Scholar",
            HeroRole::Ranger => "Ranger",
            HeroRole::Merchant => "Merchant",
            HeroRole::Cleric => "Cleric",
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

/// Name parts a hero born during play draws from (`hero_names.json`): a given
/// name and a surname combine into a proper name — "Kael Ironwood" — so an
/// era's heirs read like the seeded roster, not a string of epithets (GDD 5.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroNameBank {
    pub first_names: Vec<String>,
    pub surnames: Vec<String>,
}
