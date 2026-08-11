//! Delayed consequences (GDD 5.6): each tick counts down the scheduled
//! aftermath steps of artifact backlashes and fires those now due, mutating
//! real region and settlement state. Deterministic: no RNG.

use crate::data::strings::ChronicleText;
use crate::data::{fill, RegionBalance};
use crate::world::{
    Chronicle, ConsequenceEffect, DelayedConsequence, EventKind, Hero, Region, Settlement,
};

/// Tick down every pending consequence and fire (and remove) those now due.
#[allow(clippy::too_many_arguments)]
pub fn tick_consequences(
    pending: &mut Vec<DelayedConsequence>,
    regions: &mut [Region],
    settlements: &mut [Settlement],
    heroes: &mut [Hero],
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    for c in pending.iter_mut() {
        c.delay -= 1;
    }
    let mut i = 0;
    while i < pending.len() {
        if pending[i].delay <= 0 {
            let due = pending.remove(i);
            fire(
                &due,
                regions,
                settlements,
                heroes,
                region_balance,
                chronicle,
                text,
                year,
            );
        } else {
            i += 1;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn fire(
    c: &DelayedConsequence,
    regions: &mut [Region],
    settlements: &mut [Settlement],
    heroes: &mut [Hero],
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    match c.effect {
        // The region's largest settlement bears the blight (or reaps the bloom).
        ConsequenceEffect::SettlementBlight(prosperity) => {
            if let Some(name) = shift_largest_settlement(settlements, &c.region_id, prosperity) {
                chronicle.push(
                    year,
                    EventKind::Region,
                    fill(
                        &text.aftermath_blight,
                        &[("source", c.source.clone()), ("settlement", name)],
                    ),
                );
            }
        }
        ConsequenceEffect::SettlementBloom(prosperity) => {
            if let Some(name) = shift_largest_settlement(settlements, &c.region_id, prosperity) {
                chronicle.push(
                    year,
                    EventKind::Region,
                    fill(
                        &text.aftermath_bloom,
                        &[("source", c.source.clone()), ("settlement", name)],
                    ),
                );
            }
        }
        ConsequenceEffect::RegionUnrest { chaos, danger } => {
            if let Some(region) = regions.iter_mut().find(|r| r.id == c.region_id) {
                region.apply_deltas(0.0, chaos, danger, 0.0, region_balance);
                chronicle.push(
                    year,
                    EventKind::Region,
                    fill(
                        &text.aftermath_unrest,
                        &[
                            ("source", c.source.clone()),
                            ("region", region.name.clone()),
                        ],
                    ),
                );
            }
        }
        // The arcane shockwave rolls across the region's living heroes, dimming
        // their legends (GDD 5.6 <-> 5.4). Chronicled once if it touched anyone.
        ConsequenceEffect::HeroesShaken(renown) => {
            let mut shaken = false;
            for hero in heroes
                .iter_mut()
                .filter(|h| h.is_alive && h.region_id == c.region_id)
            {
                hero.renown = (hero.renown - renown).max(0.0);
                shaken = true;
            }
            if shaken {
                let region_name = regions
                    .iter()
                    .find(|r| r.id == c.region_id)
                    .map(|r| r.name.clone())
                    .unwrap_or_else(|| c.region_id.clone());
                chronicle.push(
                    year,
                    EventKind::Region,
                    fill(
                        &text.aftermath_heroes_shaken,
                        &[("source", c.source.clone()), ("region", region_name)],
                    ),
                );
            }
        }
    }
}

/// Shift the prosperity of the region's largest settlement by `delta` (clamped),
/// returning its name if one exists.
fn shift_largest_settlement(
    settlements: &mut [Settlement],
    region_id: &str,
    delta: f32,
) -> Option<String> {
    let target = settlements
        .iter_mut()
        .filter(|s| s.region_id == region_id)
        .max_by(|a, b| {
            a.population
                .partial_cmp(&b.population)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    target.map(|s| {
        s.prosperity = (s.prosperity + delta).clamp(0.0, 100.0);
        s.name.clone()
    })
}

#[cfg(test)]
mod tests;
