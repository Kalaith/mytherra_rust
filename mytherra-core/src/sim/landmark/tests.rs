use super::*;
use crate::data::GameData;
use crate::world::WorldState;

#[test]
fn a_flourishing_region_raises_a_wonder() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut balance = data.balance.culture.clone();
    balance.landmark_found_chance = 1.0; // guaranteed this tick
    balance.landmark_max_per_region = 100;

    let region_id = world.regions[0].id.clone();
    world.regions[0].prosperity = 90.0;
    world.regions[0].cultural_influence = 80.0;
    let region_culture = world.regions[0].culture;
    let before = world
        .landmarks
        .iter()
        .filter(|l| l.region_id == region_id)
        .count();
    let mut seq = 0;

    tick_landmark_founding(
        &mut world.landmarks,
        &world.regions,
        &mut seq,
        &data.landmark_names,
        &balance,
        &mut world.rng,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    let raised: Vec<&Landmark> = world
        .landmarks
        .iter()
        .filter(|l| l.region_id == region_id && l.id.contains("-wonder-"))
        .collect();
    assert_eq!(
        raised.len(),
        1,
        "a flourishing region should raise a wonder"
    );
    assert_eq!(
        raised[0].culture, region_culture,
        "a wonder embodies its region's culture"
    );
    let after = world
        .landmarks
        .iter()
        .filter(|l| l.region_id == region_id)
        .count();
    assert_eq!(after, before + 1);
    // Names stay unique across the map.
    let mut names: Vec<&str> = world.landmarks.iter().map(|l| l.name.as_str()).collect();
    let total = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(total, names.len(), "no two landmarks share a name");
}

#[test]
fn a_standing_wonder_grows_more_storied_with_the_ages() {
    // A wonder's cultural stature swells toward the cap the longer it stands,
    // strengthening its pull on culture — while its physical aura, tied to the
    // structure's fixed influence, never changes (GDD 5.2).
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let balance = &data.balance.culture;
    // A struggling region so no new wonder is founded during the run.
    for region in &mut world.regions {
        region.prosperity = 0.0;
        region.cultural_influence = 0.0;
    }
    world.landmarks.clear();
    world.landmarks.push(Landmark::from_seed(&LandmarkSeed {
        id: "wonder".to_owned(),
        name: "The Ancient Wonder".to_owned(),
        region_id: world.regions[0].id.clone(),
        culture: world.regions[0].culture,
        influence: 2.5,
    }));
    let mut seq = 0;

    let tick = |world: &mut WorldState, seq: &mut u64| {
        tick_landmark_founding(
            &mut world.landmarks,
            &world.regions,
            seq,
            &data.landmark_names,
            balance,
            &mut world.rng,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
    };

    tick(&mut world, &mut seq);
    let w = world.landmarks.iter().find(|l| l.id == "wonder").unwrap();
    assert!(
        w.stature > 1.0,
        "a standing wonder grows in cultural stature"
    );
    assert_eq!(w.influence, 2.5, "its physical influence is unchanged");

    // Over many ages the stature climbs to — but never past — the cap.
    for _ in 0..2000 {
        tick(&mut world, &mut seq);
    }
    let w = world.landmarks.iter().find(|l| l.id == "wonder").unwrap();
    assert!(
        (w.stature - balance.landmark_stature_cap).abs() < 1e-3,
        "an ancient wonder's stature tops out at the cap"
    );
    assert_eq!(w.influence, 2.5, "and its physical influence still holds");
}

#[test]
fn a_poor_region_raises_nothing() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut balance = data.balance.culture.clone();
    balance.landmark_found_chance = 1.0;
    // Below the prosperity/influence gates: no wonder, however lucky the roll.
    for region in &mut world.regions {
        region.prosperity = 10.0;
        region.cultural_influence = 10.0;
    }
    let before = world.landmarks.len();
    let mut seq = 0;

    tick_landmark_founding(
        &mut world.landmarks,
        &world.regions,
        &mut seq,
        &data.landmark_names,
        &balance,
        &mut world.rng,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    assert_eq!(
        world.landmarks.len(),
        before,
        "a struggling land raises no wonder"
    );
}
