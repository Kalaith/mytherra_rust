//! Per-tick region drift (GDD 5.2). Deterministic: no RNG, pure state-in /
//! state-out, so the same world always evolves the same way.

use crate::world::Region;
use macroquad_toolkit::math::approach;

/// Advance a single region by one world tick.
///
/// Prosperity drifts on chaos pressure; chaos, danger and magic slowly relax
/// toward calmer baselines when left untended, so an unmanaged world settles
/// rather than running away. Settlement / resource / magic-culture pressure
/// terms from the GDD formula are added once those systems exist.
pub fn tick_region(region: &mut Region) {
    let prosperity_delta = if region.chaos > 70.0 {
        -3.0
    } else if region.chaos < 30.0 {
        2.0
    } else {
        1.0
    };
    region.prosperity = (region.prosperity + prosperity_delta).clamp(0.0, 100.0);

    region.chaos = approach(region.chaos, 35.0, 1.0);
    region.danger = approach(region.danger, 30.0, 1.0);
    region.magic_affinity = approach(region.magic_affinity, 45.0, 0.5);

    region.refresh_status();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{ClimateType, Culture, RegionSeed};

    fn region_with(chaos: f32, prosperity: f32) -> Region {
        Region::from_seed(&RegionSeed {
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
        })
    }

    #[test]
    fn high_chaos_erodes_prosperity() {
        let mut region = region_with(80.0, 60.0);
        tick_region(&mut region);
        assert!(region.prosperity < 60.0);
    }

    #[test]
    fn calm_region_gains_prosperity() {
        let mut region = region_with(20.0, 50.0);
        tick_region(&mut region);
        assert!(region.prosperity > 50.0);
    }

    #[test]
    fn danger_relaxes_toward_baseline() {
        let mut region = region_with(50.0, 50.0);
        region.danger = 90.0;
        tick_region(&mut region);
        assert!(region.danger < 90.0);
    }
}
