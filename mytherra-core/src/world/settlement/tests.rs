use super::*;

fn balance() -> SettlementBalance {
    crate::data::GameData::load().unwrap().balance.settlement
}

#[test]
fn growth_eases_to_zero_at_carrying_capacity() {
    let capacity = 10_000.0;
    let mut s = settlement(80.0);

    s.population = 5_000.0; // half capacity
    let full = 0.05;
    let damped = s.capacity_limited_growth(full, capacity);
    assert!(
        damped > 0.0 && damped < full,
        "below capacity, growth is positive but eased: {damped}"
    );

    s.population = 10_000.0; // at capacity
    assert_eq!(s.capacity_limited_growth(full, capacity), 0.0);

    s.population = 12_000.0; // past capacity
    assert_eq!(
        s.capacity_limited_growth(full, capacity),
        0.0,
        "positive growth never carries a town past capacity"
    );

    s.population = 5_000.0; // decline from hardship still bites in full
    assert_eq!(s.capacity_limited_growth(-0.02, capacity), -0.02);
}

fn settlement(prosperity: f32) -> Settlement {
    Settlement::from_seed(&SettlementSeed {
        id: "s".to_owned(),
        name: "S".to_owned(),
        region_id: "r".to_owned(),
        population: 1000.0,
        prosperity,
    })
}

#[test]
fn prosperous_settlement_grows_faster() {
    let b = balance();
    let rich = settlement(80.0).growth_rate(70.0, 20.0, &b);
    let poor = settlement(30.0).growth_rate(30.0, 70.0, &b);
    assert!(rich > poor);
}

#[test]
fn growth_rate_is_clamped() {
    let b = balance();
    let g = settlement(100.0).growth_rate(100.0, 0.0, &b);
    assert!(g <= b.growth_max);
}

#[test]
fn thriving_settlement_contributes_positive() {
    let b = balance();
    assert!(settlement(80.0).region_contribution(&b) > 0.0);
    assert!(settlement(20.0).region_contribution(&b) < 0.0);
}

#[test]
fn tier_climbs_with_population_and_is_bounded() {
    let thresholds = [1_000.0, 5_000.0, 15_000.0, 35_000.0];
    assert_eq!(
        tier_of(300.0, &thresholds),
        0,
        "a hamlet is the smallest tier"
    );
    assert_eq!(
        tier_of(1_000.0, &thresholds),
        1,
        "meeting a threshold enters the next tier"
    );
    assert_eq!(tier_of(9_000.0, &thresholds), 2);
    assert_eq!(tier_of(20_000.0, &thresholds), 3);
    // The top tier is the highest index, never past the name count.
    assert_eq!(
        tier_of(1_000_000.0, &thresholds),
        thresholds.len(),
        "an enormous city tops out at the last tier"
    );
}
