//! Runtime resource-node state (GDD 5.3): a node whose status cycles through a
//! state machine, its output scaling with that status and feeding its region.

use crate::data::{ResourceNodeSeed, ResourceOutputs, ResourceStatus, ResourceType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceNode {
    pub id: String,
    pub name: String,
    pub region_id: String,
    pub resource_type: ResourceType,
    pub status: ResourceStatus,
}

impl ResourceNode {
    pub fn from_seed(seed: &ResourceNodeSeed) -> Self {
        Self {
            id: seed.id.clone(),
            name: seed.name.clone(),
            region_id: seed.region_id.clone(),
            resource_type: seed.resource_type,
            status: seed.status,
        }
    }

    /// Current output multiplier from the status table.
    pub fn output(&self, outputs: &ResourceOutputs) -> f32 {
        outputs.get(self.status)
    }
}
