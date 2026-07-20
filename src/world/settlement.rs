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

#[cfg(test)]
mod tests {
    use super::*;

    fn balance() -> SettlementBalance {
        crate::data::GameData::load().unwrap().balance.settlement
    }

    #[test]
    fn growth_eases_to_zero_at_carrying_capacity() {
        let capacity = 10_000.0;
        let mut s = settlement(80.0);

        s.population = 5_000.0; // half capacity
        let full = 0.05;
        let damped = s.capacity_limited_growth(full, capacity);
        assert!(
            damped > 0.0 && damped < full,
            "below capacity, growth is positive but eased: {damped}"
        );

        s.population = 10_000.0; // at capacity
        assert_eq!(s.capacity_limited_growth(full, capacity), 0.0);

        s.population = 12_000.0; // past capacity
        assert_eq!(
            s.capacity_limited_growth(full, capacity),
            0.0,
            "positive growth never carries a town past capacity"
        );

        s.population = 5_000.0; // decline from hardship still bites in full
        assert_eq!(s.capacity_limited_growth(-0.02, capacity), -0.02);
    }

    fn settlement(prosperity: f32) -> Settlement {
        Settlement::from_seed(&SettlementSeed {
            id: "s".to_owned(),
            name: "S".to_owned(),
            region_id: "r".to_owned(),
            population: 1000.0,
            prosperity,
        })
    }

    #[test]
    fn prosperous_settlement_grows_faster() {
        let b = balance();
        let rich = settlement(80.0).growth_rate(70.0, 20.0, &b);
        let poor = settlement(30.0).growth_rate(30.0, 70.0, &b);
        assert!(rich > poor);
    }

    #[test]
    fn growth_rate_is_clamped() {
        let b = balance();
        let g = settlement(100.0).growth_rate(100.0, 0.0, &b);
        assert!(g <= b.growth_max);
    }

    #[test]
    fn thriving_settlement_contributes_positive() {
        let b = balance();
        assert!(settlement(80.0).region_contribution(&b) > 0.0);
        assert!(settlement(20.0).region_contribution(&b) < 0.0);
    }
}
