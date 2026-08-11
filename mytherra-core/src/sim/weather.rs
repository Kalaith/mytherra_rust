//! Per-tick weather behaviour (GDD 5.6): natural fronts arise on their own,
//! biased by each region's climate; then every active front applies its pattern
//! scaled by remaining magnitude and decays, dissipating below the floor. Only
//! the natural spawn uses RNG — the rest is deterministic.

use crate::data::strings::ChronicleText;
use crate::data::{
    fill, ClimateType, RegionBalance, ResourceStatus, WeatherBalance, WeatherIntensity,
    WeatherPattern,
};
use crate::world::{Chronicle, EventKind, Region, ResourceNode, WeatherEvent};
use macroquad_toolkit::rng::SeededRng;

/// Advance every active weather front by one tick, and let a new natural front
/// arise from the world's climates.
#[allow(clippy::too_many_arguments)]
pub fn tick_weather(
    weather: &mut Vec<WeatherEvent>,
    regions: &mut [Region],
    nodes: &mut [ResourceNode],
    patterns: &[WeatherPattern],
    intensities: &[WeatherIntensity],
    rng: &mut SeededRng,
    balance: &WeatherBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
    era_pressure: f32,
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
        era_pressure,
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

        // A holding front works the living land beneath it (GDD 5.6 <-> 5.3):
        // the resource kinds its pattern governs are pressed a rung along their
        // ladder, the odds rising with the front's remaining force.
        if let Some(pattern) = patterns.iter().find(|p| p.id == front.pattern_id) {
            shape_resources(
                front, pattern, nodes, regions, rng, balance, chronicle, text, year,
            );
        }

        front.magnitude -= balance.decay_per_tick;
    }
    weather.retain(|f| f.magnitude >= balance.min_magnitude);
}

/// The top rung of the living ladder (Flourishing).
const TOP_RUNG: u8 = 4;

/// Where a status sits on the living ladder a front can push along, worst (0,
/// run dry) to best (flourishing). The strife states — contested, corrupted,
/// unstable — sit off this ladder and return `None`, untouched by the skies:
/// weather works the land, not the conflict upon it.
fn thriving_rung(status: ResourceStatus) -> Option<u8> {
    use ResourceStatus::*;
    match status {
        Depleted => Some(0),
        Overworked => Some(1),
        Active => Some(2),
        Blessed => Some(3),
        Flourishing => Some(TOP_RUNG),
        Contested | Corrupted | Unstable => None,
    }
}

/// The status at a rung of the living ladder.
fn from_rung(rung: u8) -> ResourceStatus {
    use ResourceStatus::*;
    match rung {
        0 => Depleted,
        1 => Overworked,
        2 => Active,
        3 => Blessed,
        _ => Flourishing,
    }
}

/// Press the resource nodes a front's pattern governs along their living ladder
/// (GDD 5.6 <-> 5.3): withered kinds slip toward ruin, quickened kinds climb
/// toward flourish, the odds scaled by the front's remaining magnitude. Only
/// nodes already on the ladder move; a node driven to its extreme — run dry or
/// brought to full flourish — is chronicled by name.
#[allow(clippy::too_many_arguments)]
fn shape_resources(
    front: &WeatherEvent,
    pattern: &WeatherPattern,
    nodes: &mut [ResourceNode],
    regions: &[Region],
    rng: &mut SeededRng,
    balance: &WeatherBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    if pattern.withers.is_empty() && pattern.quickens.is_empty() {
        return;
    }
    let chance = (balance.resource_shift_chance * front.magnitude).clamp(0.0, 1.0);
    for node in nodes.iter_mut().filter(|n| n.region_id == front.region_id) {
        let Some(rung) = thriving_rung(node.status) else {
            continue;
        };
        // A pattern never both withers and quickens one kind; each matching node
        // draws the RNG exactly once, non-matching ones not at all.
        let next = if pattern.withers.contains(&node.resource_type) {
            rng.chance(chance)
                .then(|| from_rung(rung.saturating_sub(1)))
        } else if pattern.quickens.contains(&node.resource_type) {
            rng.chance(chance)
                .then(|| from_rung((rung + 1).min(TOP_RUNG)))
        } else {
            None
        };
        let Some(next) = next.filter(|&s| s != node.status) else {
            continue; // no shift this tick, or already at the ladder's end
        };
        node.status = next;

        let line = match node.status {
            ResourceStatus::Depleted => Some(&text.weather_withered),
            ResourceStatus::Flourishing => Some(&text.weather_quickened),
            _ => None,
        };
        if let Some(line) = line {
            let region_name = regions
                .iter()
                .find(|r| r.id == node.region_id)
                .map(|r| r.name.clone())
                .unwrap_or_default();
            chronicle.push(
                year,
                EventKind::Region,
                fill(
                    line,
                    &[
                        ("pattern", pattern.name.clone()),
                        ("node", node.name.clone()),
                        ("region", region_name),
                    ],
                ),
            );
        }
    }
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
    era_pressure: f32,
) {
    if regions.is_empty() || patterns.is_empty() || weather.len() >= balance.max_active {
        return;
    }
    // The age's pressure whips up the skies: near a cataclysmic breaking, fronts
    // arise more often and turn Strong more readily (GDD 5.6 <-> 5.7).
    let turmoil = 1.0 + (era_pressure / 100.0).clamp(0.0, 1.0) * balance.pressure_weather_coeff;
    if !rng.chance(balance.natural_chance * turmoil) {
        return;
    }
    let region = &regions[rng.below(regions.len())];
    // One front per region at a time — skies don't stack on themselves.
    if weather.iter().any(|f| f.region_id == region.id) {
        return;
    }

    let pattern = pick_pattern(patterns, region.climate, rng);
    let strong = rng.chance((balance.natural_strong_chance * turmoil).min(1.0));
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
mod tests;
