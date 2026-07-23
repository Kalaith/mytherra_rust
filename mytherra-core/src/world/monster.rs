//! Runtime bestiary state (GDD 5.2): a beast stalking a region until it is slain
//! or ravages on. Beasts emerge dynamically from the wild places, so there is no
//! seed content — the runtime carries them, each drawn from a kind in the
//! bestiary (`data::MonsterType`).

use serde::{Deserialize, Serialize};

/// One beast abroad in a region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Monster {
    pub id: String,
    /// The beast's name, e.g. "The Shadow Wyrm of Kharzul".
    pub name: String,
    /// The bestiary kind this was drawn from, for its per-tick menace.
    pub type_id: String,
    /// Region currently stalked.
    pub region_id: String,
    /// How fierce the beast is now: its menace scales with this, resident hunters
    /// grind it down, and it grows if left unopposed. Slain below the floor.
    pub ferocity: f32,
    /// Ticks the beast has stalked, for the chronicle and UI.
    pub age: u32,
    /// Whether the beast has ascended into a named legendary terror — grown so
    /// fierce, unopposed for so long, that it ravages far beyond an ordinary
    /// menace and slaying it makes a legend (GDD 5.2). Set once, on crossing the
    /// apex threshold.
    #[serde(default)]
    pub apex: bool,
}
