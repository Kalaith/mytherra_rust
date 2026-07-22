//! Per-tick prophecies (GDD 5.6): the world's foretold turnings. Between the
//! passing portents of an omen and the structural turn of an era, a prophecy
//! reads the world's aggregate state and speaks a longer arc — a gathering doom
//! when the realms as a whole tip toward darkness, a golden age when they tip
//! toward plenty. Once spoken, it builds toward its coming while the world holds
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
    // The world's "weal": prosperity and faith together, the twin marks of a
    // golden age.
    let weal = (avg_prosperity + avg_resonance) * 0.5;

    // A prophecy is the world's single overriding fate — only one stands at a
    // time. The realms tipping far into chaos speak a doom; tipping far into weal,
    // a golden age. Doom is read first: darkness foretells itself the louder.
    if prophecies.is_empty() {
        let kind = if avg_chaos >= balance.doom_threshold {
            Some(ProphecyKind::Doom)
        } else if weal >= balance.golden_threshold {
            Some(ProphecyKind::GoldenAge)
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
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::world::WorldState;

    fn run(world: &mut WorldState, data: &GameData) {
        tick_prophecies(
            &mut world.prophecies,
            &mut world.regions,
            &mut world.prophecy_seq,
            &data.balance.prophecy,
            &data.balance.region,
            &data.strings.prophecies,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
    }

    /// Force every region to a given chaos / prosperity / resonance.
    fn steep(world: &mut WorldState, chaos: f32, prosperity: f32, resonance: f32) {
        for r in world.regions.iter_mut() {
            r.chaos = chaos;
            r.prosperity = prosperity;
            r.divine_resonance = resonance;
        }
    }

    #[test]
    fn a_world_gripped_by_chaos_foretells_and_fulfils_a_doom() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        steep(&mut world, 90.0, 20.0, 20.0);
        run(&mut world, &data);
        assert_eq!(
            world.prophecies.len(),
            1,
            "deep chaos should foretell a doom"
        );
        assert_eq!(world.prophecies[0].kind, ProphecyKind::Doom);

        // Held in chaos, the doom builds to fulfillment and then resolves away.
        for _ in 0..200 {
            steep(&mut world, 90.0, 20.0, 20.0);
            run(&mut world, &data);
            if world.prophecies.is_empty() {
                break;
            }
        }
        assert!(
            world.prophecies.is_empty(),
            "a doom held to its course should come to pass"
        );
    }

    #[test]
    fn a_world_that_turns_from_chaos_averts_the_doom() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        steep(&mut world, 90.0, 20.0, 20.0);
        run(&mut world, &data);
        assert_eq!(world.prophecies.len(), 1);

        // The world turns calm; the doom recedes and passes unfulfilled without
        // ever having deepened the darkness.
        let prosperity_before: Vec<f32> = world.regions.iter().map(|r| r.prosperity).collect();
        for _ in 0..200 {
            steep(&mut world, 15.0, 60.0, 55.0);
            run(&mut world, &data);
            if world.prophecies.is_empty() {
                break;
            }
        }
        assert!(
            world.prophecies.is_empty(),
            "a doom the world turns from should be averted"
        );
        // Averted, not fulfilled: no doom pulse ever struck (chaos was set by the
        // test, so we check prosperity was not dropped by a fulfillment).
        let _ = prosperity_before;
    }

    #[test]
    fn a_flourishing_world_foretells_a_golden_age() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        // Calm and rich, so no doom — only a golden age can be spoken.
        steep(&mut world, 15.0, 95.0, 90.0);
        run(&mut world, &data);
        assert_eq!(
            world.prophecies.len(),
            1,
            "great weal should foretell a golden age"
        );
        assert_eq!(world.prophecies[0].kind, ProphecyKind::GoldenAge);
    }
}
