//! Runtime noble houses (GDD 5.4): the great bloodlines the world's legends
//! found. A house arises when a hero passes into legend, gathers prestige from
//! the deeds of its living members, and passes a share of that renown to the
//! heirs born of its line across the ages — until its blood runs out and it is
//! forgotten. Houses arise dynamically, so there is no seed content; they
//! reference their members by hero id rather than owning a field on the hero.

use serde::{Deserialize, Serialize};

/// A noble house — a bloodline running through the heroes across the ages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct House {
    pub id: String,
    /// The house's name, e.g. "The House of Brogan Aldwin".
    pub name: String,
    /// The region the house was founded in — the seat its heirs are drawn back to.
    pub seat_region_id: String,
    /// The hero who founded the line, for the chronicle.
    pub founder_name: String,
    /// Ids of every hero who has belonged to the house, living or dead — its whole
    /// lineage. Living members lend their renown to the house's prestige.
    pub member_ids: Vec<String>,
    /// The house's standing, drifting toward the summed renown of its living
    /// members: it swells while the line produces the famed and fades once its
    /// blood dies out.
    pub prestige: f32,
}

impl House {
    /// Whether a hero already belongs to this house.
    pub fn holds(&self, hero_id: &str) -> bool {
        self.member_ids.iter().any(|id| id == hero_id)
    }
}
