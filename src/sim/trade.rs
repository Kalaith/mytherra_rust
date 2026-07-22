//! Per-tick trade routes (GDD 5.2): each route enriches both endpoints and
//! nudges their prosperity — and cultural influence — toward the pair's average,
//! so wealth and ideas both spread along the network. Deterministic: no RNG.

use crate::data::strings::ChronicleText;
use crate::data::{fill, HeroRole, RegionBalance, ResourceStatus, TradeBalance};
use crate::world::{Chronicle, EventKind, Hero, Region, ResourceNode, TradeRoute};
use macroquad_toolkit::rng::SeededRng;

pub fn tick_trade(
    routes: &[TradeRoute],
    regions: &mut [Region],
    heroes: &[Hero],
    resource_nodes: &[ResourceNode],
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
        // Trade thrives where there is something to trade: every producing
        // resource node at either endpoint fills the caravans further, so a road
        // between resource-rich lands carries more than one between barren ones
        // (GDD 5.2 <-> 5.3). A node run dry lends nothing.
        let goods = resource_nodes
            .iter()
            .filter(|n| {
                n.status != ResourceStatus::Depleted
                    && (n.region_id == route.region_a || n.region_id == route.region_b)
            })
            .count();
        let volume = route.volume
            + merchants as f32 * balance.merchant_volume_bonus
            + goods as f32 * balance.resource_volume_bonus;

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

/// Occasionally forge a new trade route from a prospering region to the richest
/// market it isn't yet tied to (GDD 5.2): the counterpart to settlement founding,
/// resource discovery, and landmark raising, and the way a region born
/// economically isolated — a breakaway, a conquest, a frontier — is drawn into
/// the caravan network once its own wealth rises. Wealth reaches for wealth, so a
/// new road binds the founder to the most prosperous eligible partner.
/// Deterministic given the RNG state.
#[allow(clippy::too_many_arguments)]
pub fn tick_trade_founding(
    routes: &mut Vec<TradeRoute>,
    regions: &[Region],
    seq: &mut u64,
    balance: &TradeBalance,
    rng: &mut SeededRng,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    for region in regions {
        if region.prosperity < balance.found_min_prosperity
            || route_count(routes, &region.id) >= balance.found_max_routes_per_region
        {
            continue;
        }
        if !rng.chance(balance.found_chance) {
            continue;
        }

        // Wealth reaches for wealth: bind to the most prosperous region not yet
        // tied to this one and still under its own route cap. Ties break by id so
        // the choice is deterministic.
        let partner = regions
            .iter()
            .filter(|o| {
                o.id != region.id
                    && o.prosperity >= balance.found_min_prosperity
                    && route_count(routes, &o.id) < balance.found_max_routes_per_region
                    && !connected(routes, &region.id, &o.id)
            })
            .max_by(|x, y| {
                x.prosperity
                    .total_cmp(&y.prosperity)
                    .then_with(|| x.id.cmp(&y.id))
            });
        let Some(partner) = partner else {
            continue;
        };

        *seq += 1;
        let name = fill(
            &text.trade_route_name,
            &[
                ("region_a", region.name.clone()),
                ("region_b", partner.name.clone()),
            ],
        );
        routes.push(TradeRoute {
            id: format!("route-{seq}"),
            name: name.clone(),
            region_a: region.id.clone(),
            region_b: partner.id.clone(),
            volume: balance.found_volume,
        });
        chronicle.push(
            year,
            EventKind::Region,
            fill(
                &text.trade_route_forged,
                &[
                    ("route", name),
                    ("region_a", region.name.clone()),
                    ("region_b", partner.name.clone()),
                ],
            ),
        );
    }
}

/// How many routes touch a region.
fn route_count(routes: &[TradeRoute], region_id: &str) -> usize {
    routes
        .iter()
        .filter(|r| r.region_a == region_id || r.region_b == region_id)
        .count()
}

/// Whether a route already ties these two regions together (either direction).
fn connected(routes: &[TradeRoute], a: &str, b: &str) -> bool {
    routes
        .iter()
        .any(|r| (r.region_a == a && r.region_b == b) || (r.region_a == b && r.region_b == a))
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
            &world.resource_nodes,
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
                &world.resource_nodes,
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
            &world.resource_nodes,
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
            &world.resource_nodes,
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
                &world.resource_nodes,
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

    #[test]
    fn producing_resources_swell_the_wealth_a_route_carries() {
        // A route from a resource-rich land carries more than one from barren
        // ground, and a run-dry node lends nothing (GDD 5.2 <-> 5.3). Endpoints
        // start equal, so the equalize term is zero and only the volume bonus moves
        // prosperity.
        let data = GameData::load().unwrap();
        let gain = |nodes: Vec<ResourceNode>| {
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
            world.heroes.clear(); // no merchants
            world.resource_nodes = nodes;
            let before = world.regions[ai].prosperity;
            tick_trade(
                &world.trade_routes,
                &mut world.regions,
                &world.heroes,
                &world.resource_nodes,
                &data.balance.trade,
                &data.balance.region,
            );
            world.regions[ai].prosperity - before
        };

        let node = |status: ResourceStatus| ResourceNode {
            id: "n".to_owned(),
            name: "The Vein".to_owned(),
            region_id: "aldermoor".to_owned(),
            resource_type: crate::data::ResourceType::Mine,
            status,
        };

        let with_ore = gain(vec![node(ResourceStatus::Active)]);
        let barren = gain(vec![]);
        let run_dry = gain(vec![node(ResourceStatus::Depleted)]);
        assert!(
            with_ore > barren,
            "a route from a resource-rich land should carry more ({with_ore} vs {barren})"
        );
        assert!(
            (run_dry - barren).abs() < 1e-4,
            "a depleted node lends nothing to trade"
        );
    }

    #[test]
    fn a_prospering_isolated_region_is_drawn_into_the_network() {
        // A fifth region, prosperous but tied to no road, should be bound into the
        // caravan network — to the richest eligible partner (GDD 5.2).
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut balance = data.balance.trade.clone();
        balance.found_chance = 1.0; // certain this tick
        balance.found_max_routes_per_region = 10; // don't cap in this test

        // A new, unconnected, prosperous region — as if just born of a fracture.
        let newcomer = Region {
            id: "frontier".to_owned(),
            name: "Frontier".to_owned(),
            ..world.regions[0].clone()
        };
        world.regions.push(newcomer);
        for r in &mut world.regions {
            r.prosperity = 70.0; // every land clears the founding gate
        }
        let before = world.trade_routes.len();

        tick_trade_founding(
            &mut world.trade_routes,
            &world.regions,
            &mut world.trade_seq,
            &balance,
            &mut world.rng,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );

        assert!(
            world.trade_routes.len() > before,
            "a prospering isolated region should gain at least one route"
        );
        assert!(
            world
                .trade_routes
                .iter()
                .any(|r| r.region_a == "frontier" || r.region_b == "frontier"),
            "the newcomer should be tied into the network"
        );
    }

    #[test]
    fn a_poor_region_forges_no_route() {
        // Below the prosperity gate, no road is forged however lucky the roll.
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut balance = data.balance.trade.clone();
        balance.found_chance = 1.0;
        for r in &mut world.regions {
            r.prosperity = balance.found_min_prosperity - 10.0;
        }
        let before = world.trade_routes.len();

        tick_trade_founding(
            &mut world.trade_routes,
            &world.regions,
            &mut world.trade_seq,
            &balance,
            &mut world.rng,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        assert_eq!(
            world.trade_routes.len(),
            before,
            "a struggling realm forges no new roads"
        );
    }

    #[test]
    fn a_forged_route_never_duplicates_an_existing_one() {
        // Run founding hard for many ticks; every route stays a unique unordered
        // pair, so no two regions are ever bound twice.
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut balance = data.balance.trade.clone();
        balance.found_chance = 1.0;
        balance.found_max_routes_per_region = 100;
        for r in &mut world.regions {
            r.prosperity = 90.0;
        }

        for _ in 0..50 {
            tick_trade_founding(
                &mut world.trade_routes,
                &world.regions,
                &mut world.trade_seq,
                &balance,
                &mut world.rng,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
        }

        let mut pairs: Vec<(String, String)> = world
            .trade_routes
            .iter()
            .map(|r| {
                let (x, y) = (r.region_a.clone(), r.region_b.clone());
                if x <= y {
                    (x, y)
                } else {
                    (y, x)
                }
            })
            .collect();
        let total = pairs.len();
        pairs.sort();
        pairs.dedup();
        assert_eq!(total, pairs.len(), "no two routes bind the same pair");
    }
}
