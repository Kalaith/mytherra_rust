//! Conquest (GDD 5.2): a stable region annexes a trade-linked neighbour that
//! has collapsed into crisis and has no hero to defend it, merging the loser's
//! people and holdings into the victor and removing it from the map. The inverse
//! of a fracture — it removes a region rather than adding one.

use crate::data::strings::ChronicleText;
use crate::data::{fill, ConquestBalance, RegionBalance};
use crate::world::{
    Artifact, Chronicle, EventKind, Hero, Landmark, Region, RegionAgendas, ResourceNode,
    Settlement, TradeRoute, WeatherEvent,
};

/// Does a hero strong enough to hold the region against invasion live there?
fn has_defender(heroes: &[Hero], region_id: &str, balance: &ConquestBalance) -> bool {
    heroes
        .iter()
        .any(|h| h.is_alive && h.region_id == region_id && h.level >= balance.defender_min_level)
}

/// The strongest aggressor / weakest eligible target pairing, if any conquest is
/// on. Deterministic: ranked by the might gap, ties broken toward earlier
/// regions.
fn pick(
    regions: &[Region],
    heroes: &[Hero],
    trade_routes: &[TradeRoute],
    balance: &ConquestBalance,
) -> Option<(usize, usize)> {
    if regions.len() <= balance.min_regions {
        return None;
    }
    let mut best: Option<(usize, usize, f32)> = None;
    for (ai, aggressor) in regions.iter().enumerate() {
        if aggressor.status.is_crisis() || aggressor.might(balance) < balance.aggressor_min_might {
            continue;
        }
        let a_might = aggressor.might(balance);
        for (ti, target) in regions.iter().enumerate() {
            if ti == ai || !target.status.is_crisis() {
                continue;
            }
            let gap = a_might - target.might(balance);
            if gap < balance.conquest_margin || has_defender(heroes, &target.id, balance) {
                continue;
            }
            if balance.require_trade_link
                && !trade_routes
                    .iter()
                    .any(|r| r.touches(&aggressor.id) && r.touches(&target.id))
            {
                continue;
            }
            if best.is_none_or(|(_, _, g)| gap > g) {
                best = Some((ai, ti, gap));
            }
        }
    }
    best.map(|(ai, ti, _)| (ai, ti))
}

/// Merge a crisis-stricken region into a stronger neighbour: transfer its people
/// and holdings, scar the victor with the cost of war, then remove it. At most
/// one conquest per tick.
#[allow(clippy::too_many_arguments)]
pub(super) fn run(
    regions: &mut Vec<Region>,
    settlements: &mut [Settlement],
    resource_nodes: &mut [ResourceNode],
    landmarks: &mut [Landmark],
    artifacts: &mut [Artifact],
    weather: &mut [WeatherEvent],
    heroes: &mut [Hero],
    trade_routes: &mut Vec<TradeRoute>,
    civilization: &mut Vec<RegionAgendas>,
    conquest_momentum: &mut f32,
    balance: &ConquestBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    let Some((winner_idx, loser_idx)) = pick(regions, heroes, trade_routes, balance) else {
        return;
    };

    let winner_id = regions[winner_idx].id.clone();
    let winner_name = regions[winner_idx].name.clone();
    let loser_id = regions[loser_idx].id.clone();
    let loser_name = regions[loser_idx].name.clone();
    let spoils = regions[loser_idx].population * balance.population_transfer;

    // Reassign everything the loser owned to its conqueror.
    for s in settlements.iter_mut() {
        if s.region_id == loser_id {
            s.region_id = winner_id.clone();
        }
    }
    for n in resource_nodes.iter_mut() {
        if n.region_id == loser_id {
            n.region_id = winner_id.clone();
        }
    }
    for l in landmarks.iter_mut() {
        if l.region_id == loser_id {
            l.region_id = winner_id.clone();
        }
    }
    for a in artifacts.iter_mut() {
        if a.region_id == loser_id {
            a.region_id = winner_id.clone();
        }
    }
    for w in weather.iter_mut() {
        if w.region_id == loser_id {
            w.region_id = winner_id.clone();
        }
    }
    for h in heroes.iter_mut() {
        if h.region_id == loser_id {
            h.region_id = winner_id.clone();
        }
    }
    // Trade routes fold onto the winner; any that would loop back are cut.
    for route in trade_routes.iter_mut() {
        if route.region_a == loser_id {
            route.region_a = winner_id.clone();
        }
        if route.region_b == loser_id {
            route.region_b = winner_id.clone();
        }
    }
    trade_routes.retain(|r| r.region_a != r.region_b);

    // The victor swells with absorbed population but pays the price of war.
    let winner = &mut regions[winner_idx];
    winner.population += spoils;
    winner.strife = 0.0;
    winner.apply_deltas(
        balance.winner_prosperity,
        balance.winner_chaos,
        balance.winner_danger,
        0.0,
        region_balance,
    );

    regions.retain(|r| r.id != loser_id);
    civilization.retain(|c| c.region_id != loser_id);

    // Feed the world's conquest momentum, which drives Conquest-era pressure.
    *conquest_momentum = (*conquest_momentum + balance.momentum_gain).min(balance.momentum_cap);

    chronicle.push(
        year,
        EventKind::Region,
        fill(
            &text.region_conquest,
            &[("winner", winner_name), ("loser", loser_name)],
        ),
    );
}
