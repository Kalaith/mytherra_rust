//! Per-tick trade routes (GDD 5.2): each route enriches both endpoints and
//! nudges their prosperity — and cultural influence — toward the pair's average,
//! so wealth and ideas both spread along the network. Deterministic: no RNG.

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

        // Wealth: a flat bonus plus drift toward the pair's average prosperity.
        let bonus = balance.prosperity_bonus * route.volume;
        let (pa, pb) = (regions[a].prosperity, regions[b].prosperity);
        let avg = (pa + pb) * 0.5;
        let delta_a = bonus + (avg - pa) * balance.equalize_rate;
        let delta_b = bonus + (avg - pb) * balance.equalize_rate;
        regions[a].apply_deltas(delta_a, 0.0, 0.0, 0.0, region_balance);
        regions[b].apply_deltas(delta_b, 0.0, 0.0, 0.0, region_balance);

        // Ideas: the same shape carries cultural influence along the route, so
        // connected lands grow to resemble one another.
        let culture = balance.culture_bonus * route.volume;
        let (ca, cb) = (regions[a].cultural_influence, regions[b].cultural_influence);
        let cavg = (ca + cb) * 0.5;
        regions[a].adjust_culture(culture + (cavg - ca) * balance.culture_equalize);
        regions[b].adjust_culture(culture + (cavg - cb) * balance.culture_equalize);
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

    #[test]
    fn trade_narrows_the_culture_gap() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
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
        world.regions[ai].cultural_influence = 80.0;
        world.regions[ki].cultural_influence = 20.0;
        let gap_before =
            (world.regions[ai].cultural_influence - world.regions[ki].cultural_influence).abs();
        tick_trade(
            &world.trade_routes,
            &mut world.regions,
            &data.balance.trade,
            &data.balance.region,
        );
        let gap_after =
            (world.regions[ai].cultural_influence - world.regions[ki].cultural_influence).abs();
        assert!(
            gap_after < gap_before,
            "ideas should flow along the route, narrowing the culture gap"
        );
    }
}
