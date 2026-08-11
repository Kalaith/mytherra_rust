use super::*;

#[test]
fn new_world_seeds_regions_and_year() {
    let data = GameData::load().unwrap();
    let world = WorldState::new(&data);
    assert_eq!(world.year, data.config.start_year);
    assert_eq!(world.regions.len(), data.regions.len());
    assert!(!world.chronicle.is_empty());
}

#[test]
fn summary_averages_region_stats() {
    let data = GameData::load().unwrap();
    let world = WorldState::new(&data);
    let summary = world.summary();
    assert_eq!(summary.region_count, world.regions.len());
    assert!(summary.avg_prosperity > 0.0);
    assert!(summary.total_population > 0.0);
}

#[test]
fn tenor_worsens_as_the_world_darkens() {
    let thresholds = [60.0, 35.0, 15.0, -10.0];
    let penalty = 12.0;
    let with = |prosperity: f32, danger: f32, chaos: f32, crises: usize| WorldSummary {
        avg_prosperity: prosperity,
        avg_danger: danger,
        avg_chaos: chaos,
        regions_in_crisis: crises,
        ..Default::default()
    };

    // A calm, rich world reads as a golden age (health 90 clears every bar).
    assert_eq!(with(95.0, 3.0, 2.0, 0).tenor(&thresholds, penalty), 0);
    // A troubled, crisis-stricken world sinks toward a dark age.
    let dark = with(20.0, 80.0, 70.0, 3).tenor(&thresholds, penalty);
    assert_eq!(dark, thresholds.len(), "a broken world is a dark age");
    // And the tenor is monotonic: more turmoil never improves the age.
    assert!(
        with(60.0, 40.0, 40.0, 1).tenor(&thresholds, penalty)
            >= with(80.0, 10.0, 10.0, 0).tenor(&thresholds, penalty)
    );
}
