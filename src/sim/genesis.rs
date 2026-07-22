//! Region genesis (GDD 5.2): the world map is not fixed. Three forces reshape
//! it, each in its own sibling module:
//!
//! - [`fracture`] — a region ground down by sustained chaos and danger secedes,
//!   part of it breaking away as a new region under a hero who leads the revolt.
//! - [`conquest`] — a strong region annexes a trade-linked neighbour that has
//!   collapsed into crisis and has no hero to defend it, removing the loser.
//! - [`frontier`] — a veteran hero in a thriving, populous region leads settlers
//!   out to found a new frontier region.
//!
//! The three interact rather than merely coexisting. A high-level hero in a
//! crisis-stricken region *defends* it from conquest but can instead *lead* it
//! to secede — the same catalyst pushing toward fracture and away from being
//! swallowed; the *same* calibre of hero, in a thriving land, instead founds a
//! frontier. Crisis contracts the map (fracture, conquest); success expands it
//! (frontier). Everything is deterministic bar the RNG-drawn breakaway/frontier
//! names, town defections, and the founding roll, so a given seed always
//! reshapes the same way.

mod conquest;
mod fracture;
mod frontier;

use crate::data::GameData;
use crate::world::WorldState;

/// Advance region genesis by one tick: age every region's strife, then let the
/// map reshape — at most one conquest, one founding, and one fracture per tick.
pub fn tick_genesis(world: &mut WorldState, data: &GameData) {
    let WorldState {
        regions,
        settlements,
        resource_nodes,
        landmarks,
        artifacts,
        weather,
        heroes,
        trade_routes,
        civilization,
        pacts,
        region_seq,
        conquest_momentum,
        secession_momentum,
        rng,
        chronicle,
        year,
        ..
    } = world;
    let year = *year;
    let agenda_count = data.agendas.len();
    let region_balance = &data.balance.region;
    let gtext = &data.strings.genesis;
    let ctext = &data.strings.chronicle;

    for region in regions.iter_mut() {
        fracture::accrue_strife(region, artifacts, &data.balance.genesis);
    }

    conquest::run(
        regions,
        settlements,
        resource_nodes,
        landmarks,
        artifacts,
        weather,
        heroes,
        trade_routes,
        civilization,
        pacts,
        &data.agendas,
        data.balance.civilization.apply_threshold,
        conquest_momentum,
        &data.balance.conquest,
        region_balance,
        chronicle,
        ctext,
        year,
    );

    frontier::run(
        regions,
        heroes,
        artifacts,
        civilization,
        &data.agendas,
        data.balance.civilization.apply_threshold,
        trade_routes,
        region_seq,
        agenda_count,
        rng,
        &data.balance.frontier,
        region_balance,
        chronicle,
        gtext,
        ctext,
        year,
    );

    fracture::run(
        regions,
        settlements,
        resource_nodes,
        heroes,
        civilization,
        trade_routes,
        region_seq,
        secession_momentum,
        agenda_count,
        rng,
        &data.balance.genesis,
        region_balance,
        chronicle,
        gtext,
        ctext,
        year,
    );
}

#[cfg(test)]
mod tests;
