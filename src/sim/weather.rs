//! Per-tick weather behaviour (GDD 5.6): natural fronts arise on their own,
//! biased by each region's climate; then every active front applies its pattern
//! scaled by remaining magnitude and decays, dissipating below the floor. Only
//! the natural spawn uses RNG — the rest is deterministic.

use crate::data::strings::ChronicleText;
use crate::data::{
    fill, ClimateType, RegionBalance, WeatherBalance, WeatherIntensity, WeatherPattern,
};
use crate::world::{Chronicle, EventKind, Region, WeatherEvent};
use macroquad_toolkit::rng::SeededRng;

/// Advance every active weather front by one tick, and let a new natural front
/// arise from the world's climates.
#[allow(clippy::too_many_arguments)]
pub fn tick_weather(
    weather: &mut Vec<WeatherEvent>,
    regions: &mut [Region],
    patterns: &[WeatherPattern],
    intensities: &[WeatherIntensity],
    rng: &mut SeededRng,
    balance: &WeatherBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    spawn_natural(
        weather,
        regions,
        patterns,
        intensities,
        rng,
        balance,
        chronicle,
        text,
        year,
    );

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

/// Maybe raise one natural front over a random region, its pattern drawn from
/// those its climate favours (GDD 5.6). Deterministic given the RNG state.
#[allow(clippy::too_many_arguments)]
fn spawn_natural(
    weather: &mut Vec<WeatherEvent>,
    regions: &[Region],
    patterns: &[WeatherPattern],
    intensities: &[WeatherIntensity],
    rng: &mut SeededRng,
    balance: &WeatherBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    if regions.is_empty() || patterns.is_empty() || weather.len() >= balance.max_active {
        return;
    }
    if !rng.chance(balance.natural_chance) {
        return;
    }
    let region = &regions[rng.below(regions.len())];
    // One front per region at a time — skies don't stack on themselves.
    if weather.iter().any(|f| f.region_id == region.id) {
        return;
    }

    let pattern = pick_pattern(patterns, region.climate, rng);
    let strong = rng.chance(balance.natural_strong_chance);
    let intensity_id = if strong {
        &balance.natural_strong_id
    } else {
        &balance.natural_gentle_id
    };
    let Some(intensity) = intensities.iter().find(|i| &i.id == intensity_id) else {
        return;
    };

    chronicle.push(
        year,
        EventKind::Region,
        fill(
            &text.weather_natural,
            &[
                ("intensity", intensity.name.clone()),
                ("pattern", pattern.name.clone()),
                ("region", region.name.clone()),
            ],
        ),
    );
    weather.push(WeatherEvent::from_parts(
        region.id.clone(),
        pattern,
        intensity,
    ));
}

/// Pick a pattern favoured by the climate, falling back to any pattern when the
/// climate has no signature weather.
fn pick_pattern<'a>(
    patterns: &'a [WeatherPattern],
    climate: ClimateType,
    rng: &mut SeededRng,
) -> &'a WeatherPattern {
    let matching: Vec<&WeatherPattern> = patterns
        .iter()
        .filter(|p| p.climates.contains(&climate))
        .collect();
    if matching.is_empty() {
        &patterns[rng.below(patterns.len())]
    } else {
        matching[rng.below(matching.len())]
    }
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

    /// Tick weather with natural spawning disabled, to isolate the front physics.
    fn tick_no_spawn(world: &mut WorldState, data: &GameData) {
        let mut balance = data.balance.weather.clone();
        balance.natural_chance = 0.0;
        tick_weather(
            &mut world.weather,
            &mut world.regions,
            &data.weather_patterns,
            &data.weather_intensities,
            &mut world.rng,
            &balance,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
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

    #[test]
    fn natural_weather_arises_and_stays_capped() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut ever_saw_weather = false;
        for _ in 0..200 {
            tick_weather(
                &mut world.weather,
                &mut world.regions,
                &data.weather_patterns,
                &data.weather_intensities,
                &mut world.rng,
                &data.balance.weather,
                &data.balance.region,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
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
}
