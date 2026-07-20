//! Resource-node and settlement tuning (GDD 5.3).

use crate::data::resource::ResourceStatus;
use serde::{Deserialize, Serialize};

/// Resource-node tuning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceBalance {
    pub stress_chaos: f32,
    pub stress_danger: f32,
    pub degrade_base: f32,
    pub degrade_stress: f32,
    pub recover_base: f32,
    pub improve_base: f32,
    pub contest_chaos_threshold: f32,
    pub corrupt_base: f32,
    pub corrupt_danger: f32,
    pub region_output_scale: f32,
    /// A hazardous node poisons its region, not just its ledger (GDD 5.3): a
    /// corrupted node bleeds chaos as the taint spreads, an unstable one bleeds
    /// danger. This feeds the very stress that degraded it, so a neglected node
    /// can drag its region down with it until the region is calmed.
    pub corrupted_chaos: f32,
    pub unstable_danger: f32,
    pub outputs: ResourceOutputs,
}

/// Output multiplier per resource status (GDD 5.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceOutputs {
    pub active: f32,
    pub blessed: f32,
    pub flourishing: f32,
    pub overworked: f32,
    pub contested: f32,
    pub corrupted: f32,
    pub unstable: f32,
    pub depleted: f32,
}

impl ResourceOutputs {
    pub fn get(&self, status: ResourceStatus) -> f32 {
        match status {
            ResourceStatus::Active => self.active,
            ResourceStatus::Blessed => self.blessed,
            ResourceStatus::Flourishing => self.flourishing,
            ResourceStatus::Overworked => self.overworked,
            ResourceStatus::Contested => self.contested,
            ResourceStatus::Corrupted => self.corrupted,
            ResourceStatus::Unstable => self.unstable,
            ResourceStatus::Depleted => self.depleted,
        }
    }
}

/// Settlement growth tuning (GDD 5.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementBalance {
    pub base_growth: f32,
    pub self_prosperity_div: f32,
    pub region_prosperity_div: f32,
    pub region_chaos_div: f32,
    pub growth_min: f32,
    pub growth_max: f32,
    /// Carrying capacity per point of the settlement's supporting prosperity
    /// (region prosperity + its buildings): the land feeds only so many, so
    /// growth eases to nothing as population nears capacity (GDD 5.3).
    pub capacity_per_prosperity: f32,
    pub prosperity_drift_rate: f32,
    pub region_contribution: f32,
    /// A settlement builds a new building only once its prosperity and
    /// population clear these floors (GDD 6 — buildings grow with settlements).
    pub construction_prosperity_min: f32,
    pub construction_population_min: f32,
    /// Per-tick chance an eligible settlement raises one new building.
    pub construction_chance: f32,
    /// Extra selection weight a building type gets when it matches its region's
    /// dominant culture, so a martial land forges and a mercantile one trades.
    pub culture_affinity_weight: f32,
}
