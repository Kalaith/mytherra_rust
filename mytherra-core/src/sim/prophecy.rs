//! Per-tick prophecies (GDD 5.6): the world's foretold turnings. Between the
//! passing portents of an omen and the structural turn of an era, a prophecy
//! reads the world's aggregate state and speaks a longer arc — a gathering doom
//! when the realms as a whole tip toward darkness, a golden age when they tip
//! toward plenty, an Age of Magic when they are steeped past all measure in the
//! arcane. Once spoken, it builds toward its coming while the world holds
//! that course and recedes when the world turns, so a doom can be averted and a
//! golden age let slip. Fulfilled, it nudges the world further along the road it
//! was already travelling. Deterministic: the whole cycle reads world state, no
//! RNG — one prophecy stands at a time, the world's single overriding fate.

use crate::data::strings::{ChronicleText, ProphecyNames};
use crate::data::{fill, ProphecyBalance, RegionBalance};
use crate::world::{Chronicle, EventKind, Prophecy, ProphecyKind, Region};

#[allow(clippy::too_many_arguments)]
pub fn tick_prophecies(
    prophecies: &mut Vec<Prophecy>,
    regions: &mut [Region],
    seq: &mut u64,
    balance: &ProphecyBalance,
    region_balance: &RegionBalance,
    names: &ProphecyNames,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    if regions.is_empty() {
        return;
    }
    let n = regions.len() as f32;
    let avg_chaos = regions.iter().map(|r| r.chaos).sum::<f32>() / n;
    let avg_prosperity = regions.iter().map(|r| r.prosperity).sum::<f32>() / n;
    let avg_resonance = regions.iter().map(|r| r.divine_resonance).sum::<f32>() / n;
    let avg_magic = regions.iter().map(|r| r.magic_affinity).sum::<f32>() / n;
    // The world's "weal": prosperity and faith together, the twin marks of a
    // golden age.
    let weal = (avg_prosperity + avg_resonance) * 0.5;

    // A prophecy is the world's single overriding fate — only one stands at a
    // time. The realms tipping far into chaos speak a doom; tipping far into weal,
    // a golden age; steeped past all measure in the arcane, an Age of Magic. Doom
    // is read first (darkness foretells itself the louder), then weal, then the
    // arcane tide — the rarest, spoken only over a world drowning in wonder.
    if prophecies.is_empty() {
        let kind = if avg_chaos >= balance.doom_threshold {
            Some(ProphecyKind::Doom)
        } else if weal >= balance.golden_threshold {
            Some(ProphecyKind::GoldenAge)
        } else if avg_magic >= balance.magic_threshold {
            Some(ProphecyKind::AgeOfMagic)
        } else {
            None
        };
        if let Some(kind) = kind {
            *seq += 1;
            let name = names.for_kind(kind).to_owned();
            prophecies.push(Prophecy {
                id: format!("prophecy-{seq}"),
                name: name.clone(),
                kind,
                progress: 0.0,
                foretold_year: year,
            });
            chronicle.push(
                year,
                EventKind::Region,
                fill(&text.prophecy_foretold, &[("prophecy", name)]),
            );
        }
    }

    // Advance a standing prophecy while its premise holds, let it recede when the
    // world turns aside, and collect any that resolve this tick.
    let mut fulfilled: Vec<(ProphecyKind, String)> = Vec::new();
    prophecies.retain_mut(|p| {
        let premise_holds = match p.kind {
            ProphecyKind::Doom => avg_chaos >= balance.doom_sustain,
            ProphecyKind::GoldenAge => weal >= balance.golden_sustain,
            ProphecyKind::AgeOfMagic => avg_magic >= balance.magic_sustain,
        };
        if premise_holds {
            p.progress += balance.advance_rate;
        } else {
            p.progress -= balance.recede_rate;
        }

        if p.progress >= 1.0 {
            fulfilled.push((p.kind, p.name.clone()));
            chronicle.push(
                year,
                EventKind::Region,
                fill(&text.prophecy_fulfilled, &[("prophecy", p.name.clone())]),
            );
            false
        } else if p.progress <= 0.0 {
            chronicle.push(
                year,
                EventKind::Region,
                fill(&text.prophecy_averted, &[("prophecy", p.name.clone())]),
            );
            false
        } else {
            true
        }
    });

    // A prophecy that still hangs over the world shapes it while it waits: a
    // foretold doom spreads dread that deepens the very chaos it warns of, and a
    // foretold golden age spreads hope that lifts the faith its weal is built on.
    // So a prophecy leans toward its own fulfillment — a doom the harder to escape
    // for the fear it sows, a golden age the surer to arrive for the hope it
    // kindles — yet the nudge is gentle enough that a world firmly turning can
    // slip it still. This is the whole point of a prophecy: the telling changes
    // the told. (Only one stands at a time, so this touches at most one prophecy.)
    for prophecy in prophecies.iter() {
        for region in regions.iter_mut() {
            match prophecy.kind {
                ProphecyKind::Doom => {
                    region.apply_deltas(0.0, balance.doom_dread_chaos, 0.0, 0.0, region_balance)
                }
                ProphecyKind::GoldenAge => region.add_resonance(balance.golden_hope_resonance),
                // The gathering wonder of a foretold arcane tide deepens the very
                // magic it heralds, so the age leans toward its own arrival.
                ProphecyKind::AgeOfMagic => {
                    region.add_magic_affinity(balance.magic_wonder_affinity)
                }
            }
        }
    }

    // A prophecy come to pass nudges every region further along the road it was
    // foretold on — the darkness deepens, or the plenty spreads.
    for (kind, _name) in fulfilled {
        for region in regions.iter_mut() {
            match kind {
                ProphecyKind::Doom => region.apply_deltas(
                    -balance.doom_prosperity,
                    balance.doom_chaos,
                    balance.doom_danger,
                    0.0,
                    region_balance,
                ),
                ProphecyKind::GoldenAge => {
                    region.add_cultural_influence(balance.golden_culture);
                    region.add_resonance(balance.golden_resonance);
                }
                // The arcane tide breaks over the world: magic floods every land,
                // and as the realms marvel at the wonders loosed, their prominence
                // and their faith rise with it — never a crisis lever.
                ProphecyKind::AgeOfMagic => {
                    region.add_magic_affinity(balance.age_magic_affinity);
                    region.add_cultural_influence(balance.age_magic_culture);
                    region.add_resonance(balance.age_magic_resonance);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
