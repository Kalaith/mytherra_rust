//! Runtime pestilence state (GDD 5.3): an active plague afflicting a region,
//! sapping its people and wealth until it burns out. Plagues arise dynamically
//! from squalor and crowding — there is no seed content — and spread along the
//! trade network, so the runtime carries them but no `data` counterpart does.

use serde::{Deserialize, Serialize};

/// One active plague gripping a region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plague {
    pub id: String,
    /// The pestilence's name, e.g. "The Grey Fever of Aldermoor".
    pub name: String,
    /// Region currently afflicted.
    pub region_id: String,
    /// How virulent the plague is right now; its toll scales with this, and it
    /// decays each tick until the plague burns out below the severity floor.
    pub severity: f32,
    /// Ticks the plague has raged, for the chronicle and UI.
    pub age: u32,
}
