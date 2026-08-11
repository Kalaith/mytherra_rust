//! Runtime settlement state (GDD 5.3): a town whose population grows on its own
//! and its region's prosperity, and which in turn feeds prosperity back to its
//! region.

use crate::data::{SettlementBalance, SettlementSeed};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settlement {
    pub id: String,
    pub name: String,
    pub region_id: String,
    pub population: f32,
    pub prosperity: f32,
}

impl Settlement {
    pub fn from_seed(seed: &SettlementSeed) -> Self {
        Self {
            id: seed.id.clone(),
            name: seed.name.clone(),
            region_id: seed.region_id.clone(),
            population: seed.population.max(0.0),
            prosperity: seed.prosperity.clamp(0.0, 100.0),
        }
    }

    /// Per-tick growth rate from settlement + region state, clamped (GDD 5.3).
    pub fn growth_rate(
        &self,
        region_prosperity: f32,
        region_chaos: f32,
        balance: &SettlementBalance,
    ) -> f32 {
        (balance.base_growth
            + (self.prosperity - 50.0) / balance.self_prosperity_div
            + (region_prosperity - 50.0) / balance.region_prosperity_div
            - region_chaos / balance.region_chaos_div)
            .clamp(balance.growth_min, balance.growth_max)
    }

    /// Prosperity this settlement contributes back to its region each tick.
    pub fn region_contribution(&self, balance: &SettlementBalance) -> f32 {
        (self.prosperity - 50.0) * balance.region_contribution
    }

    /// This settlement's size tier — the index into `strings.ui.settlement_tiers`
    /// (0 = the smallest) for its current population (GDD 5.3).
    pub fn tier(&self, thresholds: &[f32]) -> usize {
        tier_of(self.population, thresholds)
    }

    /// An intrinsic growth rate limited by carrying capacity (GDD 5.3): positive
    /// growth eases to zero as population nears capacity and never carries a
    /// settlement past it, while decline from hardship still bites in full — so
    /// a town swells toward the size its land can feed, then holds.
    pub fn capacity_limited_growth(&self, rate: f32, capacity: f32) -> f32 {
        if rate > 0.0 && capacity > 0.0 {
            rate * (1.0 - self.population / capacity).max(0.0)
        } else {
            rate
        }
    }
}

/// The size tier a population falls into: the count of ascending thresholds it
/// meets or exceeds. Pure so both the sim (milestone detection) and UI agree.
pub fn tier_of(population: f32, thresholds: &[f32]) -> usize {
    thresholds.iter().filter(|t| population >= **t).count()
}

#[cfg(test)]
mod tests;
