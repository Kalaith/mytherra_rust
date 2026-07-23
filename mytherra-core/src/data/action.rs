//! Divine region-action definitions (Bless / Corrupt / Guide Research).

use serde::{Deserialize, Serialize};

/// A single region nudge the player can spend favor on. The stat deltas are the
/// *base* effect at neutral resonance; the region scales them by its divine
/// resonance when the action is applied (GDD 5.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionActionDef {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Base favor cost before resonance scaling.
    pub cost: i64,
    pub prosperity: f32,
    pub chaos: f32,
    pub danger: f32,
    pub magic_affinity: f32,
}
