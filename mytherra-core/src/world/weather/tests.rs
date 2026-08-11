use super::*;

#[test]
fn stronger_intensity_costs_more() {
    assert!(weather_cost(14, 3.0, 1.0) > weather_cost(14, 1.0, 1.0));
}

#[test]
fn high_resonance_region_is_cheaper() {
    assert!(weather_cost(14, 1.0, 0.7) < weather_cost(14, 1.0, 1.3));
}
