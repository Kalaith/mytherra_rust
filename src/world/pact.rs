//! Runtime alliances (GDD 5.2): a standing pact between two regions bound by
//! kinship of culture and the ties of trade. Pacts form dynamically among the
//! like-minded and peaceful, so there is no seed content — the runtime carries
//! them. They are the amity that answers war's enmity: allies do not fall upon
//! one another, and stand the more secure for standing together.

use serde::{Deserialize, Serialize};

/// An alliance between two regions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pact {
    pub id: String,
    pub region_a: String,
    pub region_b: String,
    /// Years the alliance has stood, for the chronicle and UI.
    pub age: u32,
}

impl Pact {
    /// Whether this pact binds the two given regions, in either order.
    pub fn binds(&self, a: &str, b: &str) -> bool {
        (self.region_a == a && self.region_b == b) || (self.region_a == b && self.region_b == a)
    }
}
