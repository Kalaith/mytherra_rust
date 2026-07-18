//! Per-tick trade routes (GDD 5.2): each route enriches both endpoints and
//! nudges their prosperity toward the pair's average, so wealth spreads along
//! the network. Deterministic: no RNG.

use crate::data::{RegionBalance, TradeBalance};
use crate::world::{Region, TradeRoute};

pub fn tick_trade(
    routes: &[TradeRoute],
    regions: &mut [Region],
    balance: &TradeBalance,
    region_balance: &RegionBalance,
) {
    for route in routes {
        let Some(a) = regions.iter().position(|r| r.id == route.region_a) else {
            continue;
        };
        let Some(b) = regions.iter().position(|r| r.id == route.region_b) else {
            continue;
        };
        if a == b {
            continue;
        }

        let bonus = balance.prosperity_bonus * route.volume;
        let (pa, pb) = (regions[a].prosperity, regions[b].prosperity);
        let avg = (pa + pb) * 0.5;
        let delta_a = bonus + (avg - pa) * balance.equalize_rate;
        let delta_b = bonus + (avg - pb) * balance.equalize_rate;
        regions[a].apply_deltas(delta_a, 0.0, 0.0, 0.0, region_balance);
        regions[b].apply_deltas(delta_b, 0.0, 0.0, 0.0, region_balance);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::world::WorldState;

    #[test]
    fn trade_narrows_the_prosperity_gap() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        // Force a wide gap on the Iron Road (aldermoor <-> kharzul).
        let ai = world
            .regions
            .iter()
            .position(|r| r.id == "aldermoor")
            .unwrap();
        let ki = world
            .regions
            .iter()
            .position(|r| r.id == "kharzul")
            .unwrap();
        world.regions[ai].prosperity = 90.0;
        world.regions[ki].prosperity = 30.0;
        let gap_before = (world.regions[ai].prosperity - world.regions[ki].prosperity).abs();
        tick_trade(
            &world.trade_routes,
            &mut world.regions,
            &data.balance.trade,
            &data.balance.region,
        );
        let gap_after = (world.regions[ai].prosperity - world.regions[ki].prosperity).abs();
        assert!(gap_after < gap_before);
    }
}
