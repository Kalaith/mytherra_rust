//! Per-tick trade routes (GDD 5.2): each route enriches both endpoints and
//! nudges their prosperity — and cultural influence — toward the pair's average,
//! so wealth and ideas both spread along the network. Deterministic: no RNG.

use crate::data::{HeroRole, RegionBalance, TradeBalance};
use crate::world::{Hero, Region, TradeRoute};

pub fn tick_trade(
    routes: &[TradeRoute],
    regions: &mut [Region],
    heroes: &[Hero],
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

        // Merchants plying the road swell what its caravans carry: every living
        // Merchant hero at either endpoint adds to the route's effective volume,
        // so the wealth it spreads grows with the traders who call it home (GDD
        // 5.2 <-> 5.4) — the Merchant role's economic counterpart to a Warrior's
        // conquest might.
        let merchants = heroes
            .iter()
            .filter(|h| {
                h.is_alive
                    && h.role == HeroRole::Merchant
                    && (h.region_id == route.region_a || h.region_id == route.region_b)
            })
            .count();
        let volume = route.volume + merchants as f32 * balance.merchant_volume_bonus;

        // Wealth: a flat bonus plus drift toward the pair's average prosperity.
        // The bonus is throttled by the more perilous endpoint's danger — trade
        // falters where the road runs through a war zone (GDD 5.2).
        let peril = regions[a].danger.max(regions[b].danger);
        let safety = (1.0 - peril * balance.peril_penalty).clamp(balance.min_safety, 1.0);
        let bonus = balance.prosperity_bonus * volume * safety;
        let (pa, pb) = (regions[a].prosperity, regions[b].prosperity);
        let avg = (pa + pb) * 0.5;
        let delta_a = bonus + (avg - pa) * balance.equalize_rate;
        let delta_b = bonus + (avg - pb) * balance.equalize_rate;

        // Arcana travels the roads too: magic affinity drifts toward the pair's
        // average, so an attuned land (a manaspring's blessing, a mystical bent)
        // shares its arcane current with its trade partners (GDD 5.2 <-> 5.6).
        // Trade only spreads magic, never conjures it — no flat bonus here.
        let (ma, mb) = (regions[a].magic_affinity, regions[b].magic_affinity);
        let mavg = (ma + mb) * 0.5;
        let magic_a = (mavg - ma) * balance.magic_equalize;
        let magic_b = (mavg - mb) * balance.magic_equalize;

        regions[a].apply_deltas(delta_a, 0.0, 0.0, magic_a, region_balance);
        regions[b].apply_deltas(delta_b, 0.0, 0.0, magic_b, region_balance);

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
            &world.heroes,
            &data.balance.trade,
            &data.balance.region,
        );
        let gap_after = (world.regions[ai].prosperity - world.regions[ki].prosperity).abs();
        assert!(gap_after < gap_before);
    }

    #[test]
    fn peril_on_a_route_throttles_its_trade_income() {
        let data = GameData::load().unwrap();
        // Prosperity a safe endpoint gains from the Iron Road when its partner
        // sits at the given danger. Both endpoints start equal, so the equalize
        // term is zero and only the throttled trade bonus moves prosperity.
        let gain = |partner_danger: f32| {
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
            world.regions[ai].prosperity = 50.0;
            world.regions[ki].prosperity = 50.0;
            world.regions[ai].danger = 0.0;
            world.regions[ki].danger = partner_danger;
            let before = world.regions[ai].prosperity;
            tick_trade(
                &world.trade_routes,
                &mut world.regions,
                &world.heroes,
                &data.balance.trade,
                &data.balance.region,
            );
            world.regions[ai].prosperity - before
        };

        assert!(
            gain(0.0) > gain(100.0),
            "a route to a war-torn partner should carry less trade than a safe one"
        );
        assert!(
            gain(100.0) > 0.0,
            "even a perilous route still carries some trade (the min_safety floor)"
        );
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
            &world.heroes,
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

    #[test]
    fn trade_narrows_the_magic_gap() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        // A wide arcane gap on the Iron Road (aldermoor <-> kharzul); trade should
        // spread the attunement from the steeped land toward the barren one.
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
        world.regions[ai].magic_affinity = 80.0;
        world.regions[ki].magic_affinity = 20.0;
        let gap_before =
            (world.regions[ai].magic_affinity - world.regions[ki].magic_affinity).abs();
        tick_trade(
            &world.trade_routes,
            &mut world.regions,
            &world.heroes,
            &data.balance.trade,
            &data.balance.region,
        );
        let gap_after = (world.regions[ai].magic_affinity - world.regions[ki].magic_affinity).abs();
        assert!(
            gap_after < gap_before,
            "arcana should travel the road, narrowing the magic gap"
        );
    }

    #[test]
    fn a_merchant_hero_swells_the_wealth_a_route_carries() {
        // The same route enriches its endpoints more when a Merchant hero plies
        // it than when the land holds none (GDD 5.2 <-> 5.4). Both endpoints start
        // equal, so the equalize term is zero and only the volume-scaled bonus
        // moves prosperity.
        let data = GameData::load().unwrap();
        let gain = |role: crate::data::HeroRole| {
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
            world.regions[ai].prosperity = 50.0;
            world.regions[ki].prosperity = 50.0;
            world.regions[ai].danger = 0.0;
            world.regions[ki].danger = 0.0;
            // A single hero of the given role living at one endpoint.
            world.heroes = vec![Hero {
                id: "h".to_owned(),
                name: "H".to_owned(),
                role,
                region_id: world.regions[ai].id.clone(),
                level: 5,
                age: 30,
                is_alive: true,
                renown: 0.0,
            }];
            let before = world.regions[ai].prosperity;
            tick_trade(
                &world.trade_routes,
                &mut world.regions,
                &world.heroes,
                &data.balance.trade,
                &data.balance.region,
            );
            world.regions[ai].prosperity - before
        };

        assert!(
            gain(HeroRole::Merchant) > gain(HeroRole::Warrior),
            "a merchant should carry more wealth down the road than a warrior does"
        );
    }
}
