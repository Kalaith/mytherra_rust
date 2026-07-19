//! Frontier founding (GDD 5.2): the third genesis path and the mirror of a
//! fracture — born of prosperity, not strife. A veteran hero living in a
//! *thriving*, populous region can lead settlers out to found a new frontier
//! region, carrying a slice of the home population and settling as its first
//! champion. Where fracture and conquest are driven by crisis, expansion is
//! driven by success — so a well-tended world grows new regions of its own.

use crate::data::strings::{ChronicleText, GenesisText};
use crate::data::{fill, FrontierBalance, RegionBalance, RegionSeed};
use crate::world::{Chronicle, EventKind, Hero, Region, RegionAgendas, RegionStatus};
use macroquad_toolkit::rng::SeededRng;

/// A founding hero: a veteran living in a thriving, populous region. Scanned in
/// index order; each eligible hero rolls the founding chance, so selection is
/// deterministic for a given RNG state.
fn pick(
    regions: &[Region],
    heroes: &[Hero],
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
        if rng.chance(balance.found_chance) {
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
    let Some((hero_idx, parent_idx)) = pick(regions, heroes, rng, balance) else {
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
    let frontier_name = frontier_name(&parent_name, &hero_name, genesis_text, rng);
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

/// Choose a frontier's name from the data-driven templates.
fn frontier_name(parent: &str, hero: &str, text: &GenesisText, rng: &mut SeededRng) -> String {
    let template = rng
        .choose(&text.frontier_names)
        .cloned()
        .unwrap_or_else(|| "{parent} Colony".to_owned());
    fill(
        &template,
        &[("parent", parent.to_owned()), ("hero", hero.to_owned())],
    )
}
