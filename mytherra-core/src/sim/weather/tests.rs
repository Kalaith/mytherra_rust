use super::*;
use crate::data::{GameData, ResourceType};
use crate::world::WorldState;

fn front(region_id: &str, magnitude: f32) -> WeatherEvent {
    WeatherEvent {
        region_id: region_id.to_owned(),
        pattern_id: "rain".to_owned(),
        pattern_name: "Rains".to_owned(),
        intensity_name: "Gentle".to_owned(),
        magnitude,
        prosperity: 0.5,
        chaos: -0.2,
        danger: -0.2,
        magic: 0.0,
    }
}

/// Tick weather with natural spawning disabled, to isolate the front physics.
fn tick_no_spawn(world: &mut WorldState, data: &GameData) {
    let mut balance = data.balance.weather.clone();
    balance.natural_chance = 0.0;
    tick_weather(
        &mut world.weather,
        &mut world.regions,
        &mut world.resource_nodes,
        &data.weather_patterns,
        &data.weather_intensities,
        &mut world.rng,
        &balance,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
        0.0,
    );
}

#[test]
fn weather_decays_and_dissipates() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let region_id = world.regions[0].id.clone();
    world.weather.push(front(&region_id, 0.15));
    tick_no_spawn(&mut world, &data);
    // 0.15 - 0.08 = 0.07 < min_magnitude (0.1) -> dissipated.
    assert!(world.weather.is_empty());
}

#[test]
fn rain_raises_prosperity() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let region_id = world.regions[0].id.clone();
    let before = world.regions[0].prosperity;
    world.weather.push(front(&region_id, 3.0));
    tick_no_spawn(&mut world, &data);
    assert!(world.regions[0].prosperity > before);
}

/// Seed a region with one node of each given type, all starting Active, and
/// hold one non-decaying front over it for many ticks — isolating how the
/// front's pattern works the land (GDD 5.6 <-> 5.3).
fn run_front(pattern_id: &str, kinds: &[ResourceType]) -> Vec<(ResourceType, ResourceStatus)> {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let region_id = world.regions[0].id.clone();

    world.resource_nodes.clear();
    for (i, &kind) in kinds.iter().enumerate() {
        world.resource_nodes.push(ResourceNode {
            id: format!("node-{i}"),
            name: format!("Node {i}"),
            region_id: region_id.clone(),
            resource_type: kind,
            status: ResourceStatus::Active,
        });
    }

    let pattern = data
        .weather_patterns
        .iter()
        .find(|p| p.id == pattern_id)
        .unwrap();
    let strong = data
        .weather_intensities
        .iter()
        .find(|i| i.id == "strong")
        .unwrap();
    world.weather.clear();
    world
        .weather
        .push(WeatherEvent::from_parts(region_id, pattern, strong));

    let mut balance = data.balance.weather.clone();
    balance.natural_chance = 0.0; // isolate the held front
    balance.decay_per_tick = 0.0; // keep it holding across the run

    for _ in 0..300 {
        tick_weather(
            &mut world.weather,
            &mut world.regions,
            &mut world.resource_nodes,
            &data.weather_patterns,
            &data.weather_intensities,
            &mut world.rng,
            &balance,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
            0.0,
        );
    }
    world
        .resource_nodes
        .iter()
        .map(|n| (n.resource_type, n.status))
        .collect()
}

#[test]
fn a_drought_withers_the_farmland_it_holds_over() {
    // A lasting drought parches farmland toward ruin but leaves a mine — a
    // kind no weather governs — right where it started.
    let out = run_front("drought", &[ResourceType::Farmland, ResourceType::Mine]);
    let farm = out
        .iter()
        .find(|(t, _)| *t == ResourceType::Farmland)
        .unwrap()
        .1;
    let mine = out
        .iter()
        .find(|(t, _)| *t == ResourceType::Mine)
        .unwrap()
        .1;
    assert_eq!(
        mine,
        ResourceStatus::Active,
        "a drought must not touch a mine"
    );
    assert!(
        thriving_rung(farm).unwrap() < thriving_rung(ResourceStatus::Active).unwrap(),
        "a lasting drought should wither farmland below Active (was {farm:?})"
    );
}

#[test]
fn bloomtide_quickens_the_farmland_it_holds_over() {
    // Held bloomtide coaxes farmland up its ladder toward flourishing.
    let out = run_front("bloom", &[ResourceType::Farmland]);
    let farm = out[0].1;
    assert!(
        thriving_rung(farm).unwrap() > thriving_rung(ResourceStatus::Active).unwrap(),
        "a lasting bloomtide should quicken farmland above Active (was {farm:?})"
    );
}

#[test]
fn a_breaking_age_whips_up_the_skies() {
    // Over an identical run, a world near its era's breaking should raise more
    // (and fiercer) natural weather than one in a calm age (GDD 5.6 <-> 5.7).
    let data = GameData::load().unwrap();
    let storm_load = |pressure: f32| {
        let mut world = WorldState::new(&data);
        world.weather.clear();
        let mut total = 0usize;
        for _ in 0..300 {
            tick_weather(
                &mut world.weather,
                &mut world.regions,
                &mut world.resource_nodes,
                &data.weather_patterns,
                &data.weather_intensities,
                &mut world.rng,
                &data.balance.weather,
                &data.balance.region,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
                pressure,
            );
            total += world.weather.len(); // active fronts this tick, summed
        }
        total
    };
    let calm = storm_load(0.0);
    let breaking = storm_load(85.0);
    assert!(
        breaking > calm,
        "a breaking age should whip up more storms ({breaking} vs {calm})"
    );
}

#[test]
fn natural_weather_arises_and_stays_capped() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut ever_saw_weather = false;
    for _ in 0..200 {
        tick_weather(
            &mut world.weather,
            &mut world.regions,
            &mut world.resource_nodes,
            &data.weather_patterns,
            &data.weather_intensities,
            &mut world.rng,
            &data.balance.weather,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
            0.0,
        );
        if !world.weather.is_empty() {
            ever_saw_weather = true;
        }
        assert!(
            world.weather.len() <= data.balance.weather.max_active,
            "natural weather exceeded the active cap"
        );
    }
    assert!(ever_saw_weather, "no natural weather ever arose");
}

#[test]
fn natural_patterns_respect_climate() {
    // A frozen climate's signature weather is frost/storm, never a drought.
    let data = GameData::load().unwrap();
    let frozen: Vec<&WeatherPattern> = data
        .weather_patterns
        .iter()
        .filter(|p| p.climates.contains(&ClimateType::Frozen))
        .collect();
    assert!(!frozen.is_empty(), "no patterns favour a frozen climate");
    assert!(
        frozen.iter().all(|p| p.id != "drought"),
        "drought should not favour a frozen climate"
    );
}
