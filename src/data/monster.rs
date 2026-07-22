//! Bestiary content: the kinds of beast that stalk the wild places (GDD 5.2).

use serde::{Deserialize, Serialize};

/// A kind of beast that can emerge in a wild region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterType {
    pub id: String,
    pub name: String,
    /// Ferocity a fresh beast of this kind emerges with — how much menace it
    /// starts with, and how much slaying it takes to bring down.
    pub start_ferocity: f32,
    /// Danger it adds to its region each tick, per unit of ferocity.
    pub danger_per_tick: f32,
    /// Fraction of the region's largest settlement it razes each tick, per unit
    /// of ferocity.
    pub raid_population: f32,
    /// An arcane beast (a wyrm, a shade, a gravewight) is born of magic and
    /// stalks the attuned lands; a natural predator stalks the merely perilous
    /// wilds.
    #[serde(default)]
    pub arcane: bool,
}
