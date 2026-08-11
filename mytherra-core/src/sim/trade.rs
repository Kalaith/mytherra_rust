//! Per-tick trade routes (GDD 5.2): each route enriches both endpoints and
//! nudges their prosperity — and cultural influence — toward the pair's average,
//! so wealth and ideas both spread along the network. Deterministic: no RNG.

use crate::data::strings::ChronicleText;
use crate::data::{fill, HeroRole, RegionBalance, ResourceStatus, TradeBalance};
use crate::world::{Chronicle, EventKind, Hero, Region, ResourceNode, TradeRoute, WeatherEvent};
use macroquad_toolkit::rng::SeededRng;

#[allow(clippy::too_many_arguments)]
pub fn tick_trade(
    routes: &[TradeRoute],
    regions: &mut [Region],
    heroes: &[Hero],
    resource_nodes: &[ResourceNode],
    weather: &[WeatherEvent],
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
        // falters where the road runs through a war zone (GDD 5.2) — and by any
        // foul weather sitting over an endpoint: a storm mires the caravans and
        // closes the road for a season (GDD 5.2 <-> 5.6). Peril and storm both bite
        // the same safety, so they throttle the wealth and the grain a route
        // carries alike.
        let peril = regions[a].danger.max(regions[b].danger);
        let storm: f32 = weather
            .iter()
            .filter(|w| {
                !w.is_fair() && (w.region_id == route.region_a || w.region_id == route.region_b)
            })
            .map(|w| w.magnitude)
            .sum();
        let safety = (1.0 - peril * balance.peril_penalty - storm * balance.storm_penalty)
            .clamp(balance.min_safety, 1.0);
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

        // Grain travels the roads too: each granary drifts toward the pair's
        // average harvest, so a land with full stores feeds a hungrier partner —
        // the grain trade that is a starving region's lifeline (GDD 5.2 <-> 5.3).
        // Throttled by the same peril that throttles wealth, so a war that severs
        // a road severs the food it carried: a besieged land is left to starve
        // alone. Trade only shares food, never conjures it — no flat bonus.
        let (ha, hb) = (regions[a].harvest, regions[b].harvest);
        let havg = (ha + hb) * 0.5;
        regions[a].add_harvest((havg - ha) * balance.harvest_equalize * safety);
        regions[b].add_harvest((havg - hb) * balance.harvest_equalize * safety);

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
mod tests;
