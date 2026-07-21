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
    /// A manaspring is a wellspring of the arcane, not the granary a farm or mine
    /// is (GDD 5.3 <-> 5.6): its yield feeds the region's magic affinity rather
    /// than its prosperity, scaled by this. So an arcane resource makes a mystical
    /// land — and a corrupted manaspring drains it — giving the resource type a
    /// role beyond its economic output.
    pub manaspring_magic_scale: f32,
    /// A hazardous node poisons its region, not just its ledger (GDD 5.3): a
    /// corrupted node bleeds chaos as the taint spreads, an unstable one bleeds
    /// danger. This feeds the very stress that degraded it, so a neglected node
    /// can drag its region down with it until the region is calmed.
    pub corrupted_chaos: f32,
    pub unstable_danger: f32,
    /// Resource discovery (GDD 5.3): a prospering, populous region occasionally
    /// opens a wholly new node — the counterpart to settlement founding, and the
    /// way a frontier region born resource-barren eventually develops its own
    /// wealth. Per-region chance each tick, gated on prosperity and population,
    /// capped per region. A discovered node starts Active (output 1.0, so it adds
    /// nothing at once — only the potential to flourish), and its type follows the
    /// region's culture (`Culture::favored_resource`).
    pub discovery_chance: f32,
    pub discovery_min_prosperity: f32,
    pub discovery_min_population: f32,
    pub discovery_max_per_region: usize,
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
    /// Population below which a settlement is abandoned and removed — a town bled
    /// dry by an age of war and famine finally empties out, rather than lingering
    /// forever as a near-empty ghost town (GDD 5.3).
    pub abandon_population: f32,
    /// A prosperous, populous region founds a new town over time (GDD 5.3): each
    /// tick an eligible region rolls `found_chance`; it must be at least
    /// `found_status_min` prosperity and hold more than `found_min_region_pop`
    /// souls, and never grows past `found_max_per_region` towns. A new town starts
    /// with `found_population` settlers, drawn from the region's people.
    pub found_chance: f32,
    pub found_status_min: f32,
    pub found_min_region_pop: f32,
    pub found_max_per_region: usize,
    pub found_population: f32,
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
    /// Ascending population thresholds that sort a settlement into a size tier
    /// (GDD 5.3): with N thresholds there are N+1 tiers, named by
    /// `strings.ui.settlement_tiers`. A settlement's tier is the count of
    /// thresholds its population meets or exceeds, so crossing one — a village
    /// swelling into a town, or a city dwindling back — is a chronicled milestone.
    pub tier_thresholds: Vec<f32>,
}
