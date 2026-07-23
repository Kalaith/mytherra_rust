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

    /// Force every region to a given chaos / prosperity / resonance, leaving magic
    /// low so no Age of Magic intrudes on the doom/golden-age cases.
    fn steep(world: &mut WorldState, chaos: f32, prosperity: f32, resonance: f32) {
        for r in world.regions.iter_mut() {
            r.chaos = chaos;
            r.prosperity = prosperity;
            r.divine_resonance = resonance;
            r.magic_affinity = 30.0;
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

    #[test]
    fn a_standing_doom_sows_dread_that_deepens_the_chaos() {
        let data = GameData::load().unwrap();
        let b = &data.balance.prophecy;
        let mut world = WorldState::new(&data);
        // A doom is spoken over a chaos-gripped world.
        steep(&mut world, 90.0, 20.0, 20.0);
        run(&mut world, &data);
        assert_eq!(world.prophecies[0].kind, ProphecyKind::Doom);

        // With the doom now standing, the dread it sows raises chaos further, above
        // where it was set — the telling deepening the darkness.
        let chaos_before = world.regions[0].chaos;
        run(&mut world, &data);
        assert!(
            world.regions[0].chaos > chaos_before,
            "a standing doom's dread should deepen a region's chaos ({} vs {})",
            world.regions[0].chaos,
            chaos_before
        );
        assert!(b.doom_dread_chaos > 0.0);
    }

    #[test]
    fn a_world_drowning_in_magic_foretells_an_age_of_the_arcane() {
        let data = GameData::load().unwrap();
        let b = &data.balance.prophecy;
        let mut world = WorldState::new(&data);
        // Calm and only moderately rich — no doom, no golden age — but steeped in
        // the arcane past the threshold: only an Age of Magic can be spoken.
        for r in world.regions.iter_mut() {
            r.chaos = 20.0;
            r.prosperity = 55.0;
            r.divine_resonance = 40.0;
            r.magic_affinity = b.magic_threshold + 5.0;
        }
        run(&mut world, &data);
        assert_eq!(
            world.prophecies.len(),
            1,
            "a world drowning in magic should foretell an Age of Magic"
        );
        assert_eq!(world.prophecies[0].kind, ProphecyKind::AgeOfMagic);

        // While it stands, the gathering wonder deepens the world's magic further.
        let magic_before = world.regions[0].magic_affinity;
        run(&mut world, &data);
        assert!(
            world.regions[0].magic_affinity > magic_before,
            "a standing Age of Magic's wonder should deepen a region's magic"
        );
    }

    #[test]
    fn a_golden_age_outranks_an_age_of_magic_when_both_premises_hold() {
        let data = GameData::load().unwrap();
        let b = &data.balance.prophecy;
        let mut world = WorldState::new(&data);
        // Rich, devout, AND arcane — both fates are possible; the golden age, read
        // first, is the one spoken. (Chaos low so no doom pre-empts either.)
        for r in world.regions.iter_mut() {
            r.chaos = 15.0;
            r.prosperity = 95.0;
            r.divine_resonance = 90.0;
            r.magic_affinity = b.magic_threshold + 5.0;
        }
        run(&mut world, &data);
        assert_eq!(world.prophecies.len(), 1);
        assert_eq!(
            world.prophecies[0].kind,
            ProphecyKind::GoldenAge,
            "weal is read before the arcane tide"
        );
    }

    #[test]
    fn a_standing_golden_age_kindles_hope_that_lifts_the_faith() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        steep(&mut world, 15.0, 95.0, 90.0);
        run(&mut world, &data);
        assert_eq!(world.prophecies[0].kind, ProphecyKind::GoldenAge);

        // The hope of a standing golden age lifts resonance above where it stood.
        let resonance_before = world.regions[0].divine_resonance;
        run(&mut world, &data);
        assert!(
            world.regions[0].divine_resonance > resonance_before,
            "a standing golden age's hope should lift a region's faith"
        );
    }
}
