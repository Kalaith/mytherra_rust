//! Landmark founding and ageing (GDD 5.2): a flourishing, culturally-vibrant
//! region raises a wonder over time, and every standing wonder grows more
//! storied the longer it endures. Landmarks were the last of the world's
//! entities to stay fixed; now the map's cultural anchors grow with its fortunes,
//! the way its towns already do. A raised wonder pulls its region's culture,
//! lifts its cultural influence, and radiates the landmark aura like any other.

use crate::data::strings::ChronicleText;
use crate::data::{fill, CultureBalance, LandmarkNameBank, LandmarkSeed};
use crate::world::{Chronicle, EventKind, Landmark, Region};
use macroquad_toolkit::rng::SeededRng;

#[allow(clippy::too_many_arguments)]
pub fn tick_landmark_founding(
    landmarks: &mut Vec<Landmark>,
    regions: &[Region],
    seq: &mut u64,
    names: &LandmarkNameBank,
    balance: &CultureBalance,
    rng: &mut SeededRng,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    // Wonders grow more storied the longer they stand (GDD 5.2): each standing
    // landmark's cultural stature swells multiplicatively toward the cap, so an
    // ancient wonder anchors its region's identity far more than one raised this
    // age — while its physical aura stays that of the structure itself. Done
    // before founding, so a wonder raised this tick doesn't age before it exists.
    for landmark in landmarks.iter_mut() {
        landmark.stature = (landmark.stature * (1.0 + balance.landmark_stature_growth))
            .min(balance.landmark_stature_cap);
    }

    for region in regions {
        if region.prosperity < balance.landmark_found_prosperity
            || region.cultural_influence < balance.landmark_found_influence_min
        {
            continue;
        }
        let count = landmarks
            .iter()
            .filter(|l| l.region_id == region.id)
            .count();
        if count >= balance.landmark_max_per_region {
            continue;
        }
        if !rng.chance(balance.landmark_found_chance) {
            continue;
        }

        *seq += 1;
        let name = unique_landmark_name(landmarks, names, rng);
        landmarks.push(Landmark::from_seed(&LandmarkSeed {
            id: format!("{}-wonder-{}", region.id, *seq),
            name: name.clone(),
            region_id: region.id.clone(),
            // A wonder embodies the culture of the land that raised it.
            culture: region.culture,
            influence: balance.landmark_found_influence,
        }));
        chronicle.push(
            year,
            EventKind::Region,
            fill(
                &text.landmark_raised,
                &[("landmark", name), ("region", region.name.clone())],
            ),
        );
    }
}

/// A wonder's name from the bank (prefix + noun), unique among existing
/// landmarks. Deterministic given the RNG state.
fn unique_landmark_name(
    landmarks: &[Landmark],
    names: &LandmarkNameBank,
    rng: &mut SeededRng,
) -> String {
    if names.prefixes.is_empty() || names.nouns.is_empty() {
        return "The Nameless Wonder".to_owned();
    }
    let draw = |rng: &mut SeededRng| {
        format!(
            "{} {}",
            names.prefixes[rng.below(names.prefixes.len())],
            names.nouns[rng.below(names.nouns.len())],
        )
    };
    for _ in 0..16 {
        let candidate = draw(rng);
        if landmarks.iter().all(|l| l.name != candidate) {
            return candidate;
        }
    }
    let base = draw(rng);
    (2..)
        .map(|n| format!("{base} {n}"))
        .find(|c| landmarks.iter().all(|l| &l.name != c))
        .unwrap_or(base)
}

#[cfg(test)]
mod tests;
