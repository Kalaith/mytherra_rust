//! Runtime vassalage (GDD 5.2): the bond of a stronger region over a weaker one,
//! the political middle ground between the equal amity of a Pact and the outright
//! annexation of conquest. Where conquest devours a region in crisis and a pact
//! binds equals as friends, vassalage subordinates a weaker neighbour in
//! peacetime: the overlord takes tribute of the vassal's wealth and the vassal
//! keeps its own existence under the yoke, until it grows strong enough to throw
//! it off. A region with many vassals is an empire. Vassalages form dynamically,
//! so there is no seed content.

use serde::{Deserialize, Serialize};

/// A tributary bond: `vassal_id` renders tribute to `overlord_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vassalage {
    pub id: String,
    pub overlord_id: String,
    pub vassal_id: String,
    /// Years the bond has held, for the chronicle and UI.
    pub age: u32,
}

impl Vassalage {
    /// Whether this bond involves the given region, as overlord or as vassal.
    pub fn involves(&self, region_id: &str) -> bool {
        self.overlord_id == region_id || self.vassal_id == region_id
    }
}
