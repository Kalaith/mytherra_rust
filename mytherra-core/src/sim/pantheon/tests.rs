use super::*;
use crate::data::GameData;
use crate::world::WorldState;

#[test]
fn only_a_fresh_crest_into_wrath_is_reported() {
    let data = GameData::load().unwrap();
    let balance = &data.balance.pantheon;
    let top = balance.tiers.len();
    let mut world = WorldState::new(&data);

    // Two deities now sit at the apex; the rest stay calm.
    let apex_pressure = *balance.tiers.last().unwrap() + 5.0;
    world.pantheon[0].pressure = apex_pressure;
    world.pantheon[1].pressure = apex_pressure;

    // But only the first was below the apex last tick.
    let mut before: Vec<usize> = world.pantheon.iter().map(|d| d.tier(balance)).collect();
    before[0] = top - 1;
    before[1] = top;

    let cresting = deities_cresting(&before, &world.pantheon, balance);
    assert!(
        cresting.contains(&world.pantheon[0].name),
        "a fresh crest is reported"
    );
    assert!(
        !cresting.contains(&world.pantheon[1].name),
        "a deity already at the apex isn't re-reported"
    );
}

#[test]
fn pressure_drifts_toward_baseline() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    world.pantheon[0].pressure = 95.0;
    tick_pantheon(
        &mut world.pantheon,
        &mut world.regions,
        &data.balance.pantheon,
        &data.balance.region,
    );
    assert!(world.pantheon[0].pressure < 95.0);
}

#[test]
fn roused_deity_presses_its_domain() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    // Aurex (prosperity) at full pressure should raise prosperity.
    let idx = world.pantheon.iter().position(|d| d.id == "aurex").unwrap();
    world.pantheon[idx].pressure = 100.0;
    let before = world.regions[0].prosperity;
    tick_pantheon(
        &mut world.pantheon,
        &mut world.regions,
        &data.balance.pantheon,
        &data.balance.region,
    );
    assert!(world.regions[0].prosperity >= before);
}

#[test]
fn the_gods_press_a_faithful_region_harder_than_a_faithless_one() {
    // Two regions identical but for their divine resonance; a roused deity of
    // prosperity should lift the high-resonance land more than the deaf one
    // (GDD 5.6 <-> 5.2).
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let idx = world.pantheon.iter().position(|d| d.id == "aurex").unwrap();
    world.pantheon[idx].pressure = 100.0;

    // Isolate two regions with the same starting prosperity, opposite faith.
    world.regions.truncate(2);
    for r in &mut world.regions {
        r.prosperity = 50.0;
    }
    world.regions[0].divine_resonance = 100.0; // steeped in the divine
    world.regions[1].divine_resonance = 0.0; // deaf to the gods

    tick_pantheon(
        &mut world.pantheon,
        &mut world.regions,
        &data.balance.pantheon,
        &data.balance.region,
    );

    let faithful_gain = world.regions[0].prosperity - 50.0;
    let faithless_gain = world.regions[1].prosperity - 50.0;
    assert!(
        faithful_gain > faithless_gain,
        "the divine should shape the faithful land more: {faithful_gain} vs {faithless_gain}"
    );
}

#[test]
fn an_agitated_rival_provokes_its_nemesis() {
    let data = GameData::load().unwrap();
    let baseline = data.balance.pantheon.drift_target;

    // Tick the first deity with its rival calm vs. inflamed, holding every
    // region neutral so only the rivalry coupling differs between the runs.
    let run = |rival_pressure: f32| {
        let mut w = WorldState::new(&data);
        for r in &mut w.regions {
            r.prosperity = 50.0;
            r.chaos = 50.0;
            r.danger = 50.0;
            r.magic_affinity = 50.0;
        }
        w.pantheon[0].pressure = baseline;
        let rival_id = w.pantheon[0].rival_id.clone();
        if let Some(rival) = w.pantheon.iter_mut().find(|d| d.id == rival_id) {
            rival.pressure = rival_pressure;
        }
        tick_pantheon(
            &mut w.pantheon,
            &mut w.regions,
            &data.balance.pantheon,
            &data.balance.region,
        );
        w.pantheon[0].pressure
    };

    assert!(
        run(90.0) > run(40.0),
        "an agitated rival should provoke its nemesis"
    );
}

#[test]
fn an_ascendant_domain_rouses_its_deity() {
    let data = GameData::load().unwrap();
    let baseline = data.balance.pantheon.drift_target;

    // Mordath holds domain over danger. A world steeped in danger should pull
    // its pressure above the calm baseline...
    let mut dangerous = WorldState::new(&data);
    let idx = dangerous
        .pantheon
        .iter()
        .position(|d| d.effect_stat == PantheonStat::Danger)
        .unwrap();
    for r in &mut dangerous.regions {
        r.danger = 95.0;
    }
    dangerous.pantheon[idx].pressure = baseline;
    tick_pantheon(
        &mut dangerous.pantheon,
        &mut dangerous.regions,
        &data.balance.pantheon,
        &data.balance.region,
    );

    // ...while a placid world lets it settle back down.
    let mut calm = WorldState::new(&data);
    for r in &mut calm.regions {
        r.danger = 5.0;
    }
    calm.pantheon[idx].pressure = baseline;
    tick_pantheon(
        &mut calm.pantheon,
        &mut calm.regions,
        &data.balance.pantheon,
        &data.balance.region,
    );

    assert!(dangerous.pantheon[idx].pressure > baseline);
    assert!(calm.pantheon[idx].pressure < baseline);
    assert!(dangerous.pantheon[idx].pressure > calm.pantheon[idx].pressure);
}

#[test]
fn a_devout_world_rouses_the_whole_pantheon() {
    // Holding every domain stat neutral so only faith differs, a world made
    // faithful stirs the gods above their resting baseline while a faithless
    // one lets them settle (GDD 5.6 <-> 5.1).
    let data = GameData::load().unwrap();
    let baseline = data.balance.pantheon.drift_target;

    let roused = |resonance: f32| {
        let mut world = WorldState::new(&data);
        for r in &mut world.regions {
            r.prosperity = 50.0;
            r.chaos = 50.0;
            r.danger = 50.0;
            r.magic_affinity = 50.0;
            r.divine_resonance = resonance;
        }
        for d in &mut world.pantheon {
            d.pressure = baseline;
        }
        tick_pantheon(
            &mut world.pantheon,
            &mut world.regions,
            &data.balance.pantheon,
            &data.balance.region,
        );
        world.pantheon.iter().map(|d| d.pressure).sum::<f32>() / world.pantheon.len() as f32
    };

    assert!(
        roused(100.0) > roused(0.0),
        "a devout world should rouse the gods more than a faithless one"
    );
}

#[test]
fn every_ally_and_rival_id_resolves() {
    // The ally/rival web is hand-wired; a typo would silently render as a raw
    // id in the UI. Guard that every reference points at a real deity.
    let data = GameData::load().unwrap();
    let world = WorldState::new(&data);
    let ids: Vec<&str> = world.pantheon.iter().map(|d| d.id.as_str()).collect();
    for deity in &world.pantheon {
        assert!(
            ids.contains(&deity.ally_id.as_str()),
            "{} has unknown ally {}",
            deity.id,
            deity.ally_id
        );
        assert!(
            ids.contains(&deity.rival_id.as_str()),
            "{} has unknown rival {}",
            deity.id,
            deity.rival_id
        );
        assert_ne!(deity.ally_id, deity.id, "{} allies itself", deity.id);
        assert_ne!(deity.rival_id, deity.id, "{} rivals itself", deity.id);
    }
}
