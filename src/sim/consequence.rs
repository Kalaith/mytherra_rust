//! Delayed consequences (GDD 5.6): each tick counts down the scheduled
//! aftermath steps of artifact backlashes and fires those now due, mutating
//! real region and settlement state. Deterministic: no RNG.

use crate::data::strings::ChronicleText;
use crate::data::{fill, RegionBalance};
use crate::world::{
    Chronicle, ConsequenceEffect, DelayedConsequence, EventKind, Region, Settlement,
};

/// Tick down every pending consequence and fire (and remove) those now due.
#[allow(clippy::too_many_arguments)]
pub fn tick_consequences(
    pending: &mut Vec<DelayedConsequence>,
    regions: &mut [Region],
    settlements: &mut [Settlement],
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

fn fire(
    c: &DelayedConsequence,
    regions: &mut [Region],
    settlements: &mut [Settlement],
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
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::world::WorldState;

    #[test]
    fn a_scheduled_consequence_fires_only_once_its_delay_elapses() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let region_id = world.regions[0].id.clone();
        let settlement_idx = world
            .settlements
            .iter()
            .position(|s| s.region_id == region_id)
            .expect("region has a settlement");
        let before = world.settlements[settlement_idx].prosperity;

        world.pending_consequences.push(DelayedConsequence {
            region_id,
            source: "The Test Relic".to_owned(),
            delay: 2,
            effect: ConsequenceEffect::SettlementBlight(-10.0),
        });

        // delay 2 -> 1, not yet due.
        tick_consequences(
            &mut world.pending_consequences,
            &mut world.regions,
            &mut world.settlements,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        assert_eq!(world.settlements[settlement_idx].prosperity, before);
        assert_eq!(world.pending_consequences.len(), 1);

        // delay 1 -> 0, fires and is removed.
        tick_consequences(
            &mut world.pending_consequences,
            &mut world.regions,
            &mut world.settlements,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        assert!(world.settlements[settlement_idx].prosperity < before);
        assert!(world.pending_consequences.is_empty());
    }

    #[test]
    fn a_bloom_raises_the_largest_settlements_prosperity() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let region_id = world.regions[0].id.clone();
        // Target the region's largest settlement, and leave it room to grow.
        let idx = world
            .settlements
            .iter()
            .enumerate()
            .filter(|(_, s)| s.region_id == region_id)
            .max_by(|(_, a), (_, b)| a.population.total_cmp(&b.population))
            .map(|(i, _)| i)
            .expect("region has a settlement");
        world.settlements[idx].prosperity = 50.0;
        let before = world.settlements[idx].prosperity;

        world.pending_consequences.push(DelayedConsequence {
            region_id,
            source: "Bloomtide".to_owned(),
            delay: 1,
            effect: ConsequenceEffect::SettlementBloom(10.0),
        });
        tick_consequences(
            &mut world.pending_consequences,
            &mut world.regions,
            &mut world.settlements,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        assert!(world.settlements[idx].prosperity > before);
        assert!(world.pending_consequences.is_empty());
    }
}
