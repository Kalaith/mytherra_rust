//! Runtime saints (GDD 5.1 <-> 5.4): the world's venerated dead. Where a House
//! carries a bloodline forward through the living and an Order binds the living of
//! a calling, a Saint is the third legacy — the memory of a great soul who has
//! died, kept alive by the devotion of the land that raised them. When one of the
//! holy (a Cleric of high renown) or one of the truly legendary passes, the
//! faithful of their home region venerate them, and their remembered example
//! hallows that land for as long as the memory endures — fading, over the ages,
//! into the mundane past. Saints arise dynamically, so there is no seed content.

use serde::{Deserialize, Serialize};

/// A venerated soul, hallowing the land that remembers them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Saint {
    pub id: String,
    /// The saint's venerated name, e.g. "Saint Corvin Tide".
    pub name: String,
    /// The id of the hero canonized, so the same soul is never sainted twice.
    pub hero_id: String,
    /// The land that venerates them and is hallowed by their memory.
    pub region_id: String,
    /// The strength of the veneration, fading over the ages: it begins high at
    /// canonization and ebbs each tick until the saint passes from living memory
    /// into the mundane past. While it lasts it raises the region's divine
    /// resonance in measure of its strength.
    pub veneration: f32,
    /// The year the soul was raised to sainthood, for the chronicle.
    pub canonized_year: u32,
}
