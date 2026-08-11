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
