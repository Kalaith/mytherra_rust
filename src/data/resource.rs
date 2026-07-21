//! Resource-node content types: the kind of node and its state-machine status
//! (GDD 5.3).

use serde::{Deserialize, Serialize};

/// What a resource node yields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Farmland,
    Mine,
    Forest,
    Fishery,
    Quarry,
    Manaspring,
}

impl ResourceType {
    /// Display name, used when prospectors open a newly discovered node.
    pub fn label(self) -> &'static str {
        match self {
            ResourceType::Farmland => "Farmland",
            ResourceType::Mine => "Mine",
            ResourceType::Forest => "Woodland",
            ResourceType::Fishery => "Fishery",
            ResourceType::Quarry => "Quarry",
            ResourceType::Manaspring => "Manaspring",
        }
    }
}

/// A node's position in its status state machine (GDD 5.3). Output scales from
/// depleted (nothing) up to flourishing (best).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceStatus {
    Active,
    Blessed,
    Flourishing,
    Overworked,
    Contested,
    Corrupted,
    Unstable,
    Depleted,
}

impl ResourceStatus {
    pub fn label(self) -> &'static str {
        match self {
            ResourceStatus::Active => "Active",
            ResourceStatus::Blessed => "Blessed",
            ResourceStatus::Flourishing => "Flourishing",
            ResourceStatus::Overworked => "Overworked",
            ResourceStatus::Contested => "Contested",
            ResourceStatus::Corrupted => "Corrupted",
            ResourceStatus::Unstable => "Unstable",
            ResourceStatus::Depleted => "Depleted",
        }
    }
}

/// A seeded resource node (`resource_nodes.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceNodeSeed {
    pub id: String,
    pub name: String,
    pub region_id: String,
    pub resource_type: ResourceType,
    pub status: ResourceStatus,
}
