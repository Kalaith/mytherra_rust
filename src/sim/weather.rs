//! Per-tick weather behaviour (GDD 5.6): each active front applies its pattern
//! to its region scaled by remaining magnitude, then decays; fronts below the
//! magnitude floor dissipate. Deterministic: no RNG.

use crate::data::{RegionBalance, WeatherBalance};
use crate::world::{Region, WeatherEvent};

/// Advance every active weather front by one tick.
pub fn tick_weather(
    weather: &mut Vec<WeatherEvent>,
    regions: &mut [Region],
    balance: &WeatherBalance,
    region_balance: &RegionBalance,
) {
    for front in weather.iter_mut() {
        if let Some(region) = regions.iter_mut().find(|r| r.id == front.region_id) {
            let m = front.magnitude;
            region.apply_deltas(
                front.prosperity * m,
                front.chaos * m,
                front.danger * m,
                front.magic * m,
                region_balance,
            );
        }
        front.magnitude -= balance.decay_per_tick;
    }
    weather.retain(|f| f.magnitude >= balance.min_magnitude);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
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

    #[test]
    fn weather_decays_and_dissipates() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        world
            .weather
            .push(front(&world.regions[0].id.clone(), 0.15));
        tick_weather(
            &mut world.weather,
            &mut world.regions,
            &data.balance.weather,
            &data.balance.region,
        );
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
        tick_weather(
            &mut world.weather,
            &mut world.regions,
            &data.balance.weather,
            &data.balance.region,
        );
        assert!(world.regions[0].prosperity > before);
    }
}
