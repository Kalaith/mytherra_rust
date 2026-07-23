//! Runtime festivals (GDD 5.2 <-> 6): the world's great celebrations. Where the
//! crisis systems — plague, famine, war, the beasts — mark a land at its worst, a
//! Festival marks it at its best: once in a generation the world's foremost realm,
//! flourishing and at peace, throws open its gates for a grand celebration the age
//! remembers. While it lasts it draws the world's eye — deepening the host's
//! cultural renown and its faith, and crowning the heroes who dwell there with the
//! honour of the games and rites — and then it passes into memory. The constructive
//! mirror of the crisis web. Festivals arise dynamically, so there is no seed
//! content.

use serde::{Deserialize, Serialize};

/// A great celebration held by a flourishing realm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Festival {
    pub id: String,
    /// The festival's name, e.g. "the Grand Jubilee".
    pub name: String,
    /// The land holding the celebration and lifted by it.
    pub region_id: String,
    /// Years of celebration left before the festival passes into memory; it counts
    /// down each tick and the festival is remembered and removed when it reaches
    /// zero.
    pub remaining: u32,
    /// The year the festival began, for the chronicle.
    pub began_year: u32,
}
