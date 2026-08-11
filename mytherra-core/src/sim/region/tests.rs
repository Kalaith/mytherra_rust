use super::*;
use crate::data::{Balance, ClimateType, Culture, GameData, RegionSeed};

fn balance() -> Balance {
    GameData::load().unwrap().balance
}

fn region_with(chaos: f32, prosperity: f32, balance: &Balance) -> Region {
    Region::from_seed(
        &RegionSeed {
            id: "t".to_owned(),
            name: "T".to_owned(),
            climate: ClimateType::Temperate,
            culture: Culture::Martial,
            prosperity,
            chaos,
            danger: 50.0,
            magic_affinity: 50.0,
            population: 1000.0,
            cultural_influence: 50.0,
            divine_resonance: 50.0,
        },
        &balance.region,
    )
}

#[test]
fn high_chaos_erodes_prosperity() {
    let b = balance();
    let mut region = region_with(80.0, 60.0, &b);
    tick_region(&mut region, &b.region);
    assert!(region.prosperity < 60.0);
}

#[test]
fn calm_region_gains_prosperity() {
    let b = balance();
    let mut region = region_with(20.0, 50.0, &b);
    tick_region(&mut region, &b.region);
    assert!(region.prosperity > 50.0);
}

#[test]
fn danger_relaxes_toward_baseline() {
    let b = balance();
    let mut region = region_with(50.0, 50.0, &b);
    region.danger = 90.0;
    tick_region(&mut region, &b.region);
    assert!(region.danger < 90.0);
}

#[test]
fn climate_shapes_the_danger_a_region_settles_toward() {
    let b = balance();
    let mut frozen = region_with(35.0, 50.0, &b);
    frozen.climate = ClimateType::Frozen;
    let mut coastal = region_with(35.0, 50.0, &b);
    coastal.climate = ClimateType::Coastal;

    // Let each drift to its climate's equilibrium.
    for _ in 0..40 {
        tick_region(&mut frozen, &b.region);
        tick_region(&mut coastal, &b.region);
    }

    assert!(
        frozen.danger > coastal.danger,
        "a frozen waste should settle more dangerous than a mild coast ({} vs {})",
        frozen.danger,
        coastal.danger
    );
}
