//! Frontier founding (GDD 5.2): the third genesis path and the mirror of a
//! fracture — born of prosperity, not strife. A veteran hero living in a
//! *thriving*, populous region can lead settlers out to found a new frontier
//! region, carrying a slice of the home population and settling as its first
//! champion. Where fracture and conquest are driven by crisis, expansion is
//! driven by success — so a well-tended world grows new regions of its own.

use crate::data::strings::{ChronicleText, GenesisText};
use crate::data::{fill, ArtifactFocus, FrontierBalance, RegionBalance, RegionSeed};
use crate::world::{Artifact, Chronicle, EventKind, Hero, Region, RegionAgendas, RegionStatus};
use macroquad_toolkit::rng::SeededRng;

/// Founding-chance bonus a region draws from Prosperity artifacts bound to it —
/// the player's expansion lever (GDD 5.6 ↔ 5.2).
fn prosperity_bonus(region_id: &str, artifacts: &[Artifact], balance: &FrontierBalance) -> f32 {
    artifacts
        .iter()
        .filter(|a| a.focus == ArtifactFocus::Prosperity && a.region_id == region_id)
        .map(|a| a.power as f32 * balance.artifact_prosperity_chance)
        .sum()
}

/// A founding hero: a veteran living in a thriving, populous region. Scanned in
/// index order; each eligible hero rolls the founding chance, so selection is
/// deterministic for a given RNG state.
fn pick(
    regions: &[Region],
    heroes: &[Hero],
    artifacts: &[Artifact],
    rng: &mut SeededRng,
    balance: &FrontierBalance,
) -> Option<(usize, usize)> {
    for (hi, hero) in heroes.iter().enumerate() {
        if !hero.is_alive || hero.level < balance.founder_min_level {
            continue;
        }
        let Some(ri) = regions.iter().position(|r| r.id == hero.region_id) else {
            continue;
        };
        let region = &regions[ri];
        if region.status != RegionStatus::Thriving
            || region.population < balance.parent_min_population
        {
            continue;
        }
        let chance = balance.found_chance + prosperity_bonus(&region.id, artifacts, balance);
        if rng.chance(chance) {
            return Some((hi, ri));
        }
    }
    None
}

/// Found a new frontier region if a veteran in a thriving land answers the call.
/// At most one founding per tick, and never past the region cap.
#[allow(clippy::too_many_arguments)]
pub(super) fn run(
    regions: &mut Vec<Region>,
    heroes: &mut [Hero],
    artifacts: &[Artifact],
    civ: &mut Vec<RegionAgendas>,
    region_seq: &mut u64,
    agenda_count: usize,
    rng: &mut SeededRng,
    balance: &FrontierBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    genesis_text: &GenesisText,
    chronicle_text: &ChronicleText,
    year: u32,
) {
    if regions.len() >= balance.max_regions {
        return;
    }
    let Some((hero_idx, parent_idx)) = pick(regions, heroes, artifacts, rng, balance) else {
        return;
    };

    *region_seq += 1;
    let seq = *region_seq;

    let parent = &mut regions[parent_idx];
    let parent_id = parent.id.clone();
    let parent_name = parent.name.clone();
    let settlers = parent.population * balance.settler_fraction;
    parent.population = (parent.population - settlers).max(0.0);
    let climate = parent.climate;
    let culture = parent.culture;
    let magic = parent.magic_affinity * balance.child_magic_carry;

    let hero_name = heroes[hero_idx].name.clone();
    let frontier_id = format!("{parent_id}-frontier-{seq}");
    let frontier_name = frontier_name(&parent_name, &hero_name, regions, genesis_text, rng);
    let child_seed = RegionSeed {
        id: frontier_id.clone(),
        name: frontier_name.clone(),
        climate,
        // Settlers carry their home culture out to the frontier.
        culture,
        prosperity: balance.child_prosperity,
        chaos: balance.child_chaos,
        danger: balance.child_danger,
        magic_affinity: magic,
        population: settlers,
        cultural_influence: balance.child_cultural_influence,
        divine_resonance: balance.child_resonance,
    };

    // The founder settles the new land as its first champion.
    heroes[hero_idx].region_id = frontier_id.clone();

    let child = Region::from_seed(&child_seed, region_balance);
    regions.push(child);
    civ.push(RegionAgendas::new(frontier_id, agenda_count));

    chronicle.push(
        year,
        EventKind::Region,
        fill(
            &chronicle_text.region_founded,
            &[
                ("founder", hero_name),
                ("frontier", frontier_name),
                ("parent", parent_name),
            ],
        ),
    );
}

/// Choose a frontier's name from the data-driven templates, disambiguated so no
/// two regions on the map ever share a name.
fn frontier_name(
    parent: &str,
    hero: &str,
    regions: &[Region],
    text: &GenesisText,
    rng: &mut SeededRng,
) -> String {
    let template = rng
        .choose(&text.frontier_names)
        .cloned()
        .unwrap_or_else(|| "{parent} Colony".to_owned());
    let base = fill(
        &template,
        &[("parent", parent.to_owned()), ("hero", hero.to_owned())],
    );
    make_unique(base, regions)
}

/// Ensure a name is unique on the map: a name a prior region already claimed
/// gets an ascending ordinal, so two "Aldermoor Frontier"s become that and
/// "Aldermoor Frontier II". Deterministic — no RNG. Shared with the fracture
/// path, which faces the same collision.
pub(super) fn make_unique(base: String, regions: &[Region]) -> String {
    if regions.iter().all(|r| r.name != base) {
        return base;
    }
    (2..)
        .map(|n| format!("{base} {}", roman(n)))
        .find(|candidate| regions.iter().all(|r| &r.name != candidate))
        .expect("an unused ordinal always exists")
}

/// A Roman numeral for a small ordinal (frontiers of one parent stay few).
fn roman(mut n: u32) -> String {
    const VALS: [(u32, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut s = String::new();
    for (v, sym) in VALS {
        while n >= v {
            s.push_str(sym);
            n -= v;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{ClimateType, Culture, GameData};

    fn named(name: &str) -> Region {
        Region::from_seed(
            &RegionSeed {
                id: name.to_owned(),
                name: name.to_owned(),
                climate: ClimateType::Temperate,
                culture: Culture::Pastoral,
                prosperity: 50.0,
                chaos: 20.0,
                danger: 20.0,
                magic_affinity: 40.0,
                population: 1000.0,
                cultural_influence: 40.0,
                divine_resonance: 50.0,
            },
            &GameData::load().unwrap().balance.region,
        )
    }

    #[test]
    fn roman_numerals_render() {
        assert_eq!(roman(2), "II");
        assert_eq!(roman(4), "IV");
        assert_eq!(roman(9), "IX");
        assert_eq!(roman(14), "XIV");
    }

    #[test]
    fn make_unique_appends_an_ordinal_only_on_collision() {
        let regions = vec![named("Aldermoor Frontier")];
        // A free name is left as-is.
        assert_eq!(
            make_unique("Sylvan Reach".to_owned(), &regions),
            "Sylvan Reach"
        );
        // A taken name gains an ordinal...
        assert_eq!(
            make_unique("Aldermoor Frontier".to_owned(), &regions),
            "Aldermoor Frontier II"
        );
        // ...and climbs past further collisions.
        let regions = vec![named("Aldermoor Frontier"), named("Aldermoor Frontier II")];
        assert_eq!(
            make_unique("Aldermoor Frontier".to_owned(), &regions),
            "Aldermoor Frontier III"
        );
    }
}
