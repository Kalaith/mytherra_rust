//! Runtime inter-region war (GDD 5.2): a prolonged conflict between two regions,
//! draining both until one prevails or both exhaust. Wars ignite dynamically from
//! belligerence and envy, so there is no seed content — the runtime carries them.

use serde::{Deserialize, Serialize};

/// A war raging between two regions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct War {
    pub id: String,
    /// The region that declared the war, and the one it fell upon.
    pub aggressor_id: String,
    pub defender_id: String,
    /// How fiercely the war is being waged now; its toll scales with this, and it
    /// wanes each tick as both sides tire, ending when it burns below the floor.
    pub intensity: f32,
    /// Years the war has raged, for the chronicle and UI.
    pub age: u32,
}
