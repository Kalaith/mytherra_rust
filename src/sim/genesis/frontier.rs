//! Frontier founding (GDD 5.2): the third genesis path and the mirror of a
//! fracture — born of prosperity, not strife. A veteran hero living in a
//! *thriving*, populous region can lead settlers out to found a new frontier
//! region, carrying a slice of the home population and settling as its first
//! champion. Where fracture and conquest are driven by crisis, expansion is
//! driven by success — so a well-tended world grows new regions of its own.

use crate::data::strings::{ChronicleText, GenesisText};
use crate::data::{
    fill, Agenda, ArtifactFocus, FrontierBalance, HeroRole, RegionBalance, RegionSeed,
};
use crate::world::{
    Artifact, Chronicle, EventKind, Hero, Region, RegionAgendas, RegionStatus, TradeRoute,
};
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

/// Founding-chance bonus a region gains when its prevailing civilization course
/// is Expansion (GDD 5.6 <-> 5.2): a people set on expansion strike out to found
/// frontiers more readily.
fn expansion_bonus(
    region: &Region,
    civ: &[RegionAgendas],
    agendas: &[Agenda],
    apply_threshold: f32,
    balance: &FrontierBalance,
) -> f32 {
    let Some(entry) = civ.iter().find(|c| c.region_id == region.id) else {
        return 0.0;
    };
    match crate::world::dominant_agenda(agendas, region, entry, apply_threshold) {
        Some(i) if agendas[i].id == "expansion" => balance.expansion_found_chance,
        _ => 0.0,
    }
}

/// Founding-chance bonus a region gains from the living Rangers dwelling in it —
/// the pathfinders who scout the way to new land (GDD 5.2 <-> 5.4).
fn ranger_bonus(region_id: &str, heroes: &[Hero], balance: &FrontierBalance) -> f32 {
    let rangers = heroes
        .iter()
        .filter(|h| h.is_alive && h.role == HeroRole::Ranger && h.region_id == region_id)
        .count();
    rangers as f32 * balance.ranger_found_chance
}

/// A founding hero: a veteran living in a thriving, populous region. Scanned in
/// index order; each eligible hero rolls the founding chance, so selection is
/// deterministic for a given RNG state.
#[allow(clippy::too_many_arguments)]
fn pick(
    regions: &[Region],
    heroes: &[Hero],
    artifacts: &[Artifact],
    civ: &[RegionAgendas],
    agendas: &[Agenda],
    apply_threshold: f32,
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
        let chance = balance.found_chance
            + prosperity_bonus(&region.id, artifacts, balance)
            + expansion_bonus(region, civ, agendas, apply_threshold, balance)
            + ranger_bonus(&region.id, heroes, balance);
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
    agendas: &[Agenda],
    apply_threshold: f32,
    trade_routes: &mut Vec<TradeRoute>,
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
    let Some((hero_idx, parent_idx)) = pick(
        regions,
        heroes,
        artifacts,
        civ,
        agendas,
        apply_threshold,
        rng,
        balance,
    ) else {
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
    civ.push(RegionAgendas::new(frontier_id.clone(), agenda_count));

    // Wire the colony into the trade network with a busy road home, so it isn't
    // born economically marooned: it shares in trade wealth and culture at once,
    // and — a trade link being conquest's precondition — remains part of the
    // geopolitics rather than an untouchable island (GDD 5.2).
    trade_routes.push(TradeRoute {
        id: format!("route-{parent_id}-{frontier_id}"),
        name: format!("{frontier_name} Road"),
        region_a: parent_id.clone(),
        region_b: frontier_id,
        volume: balance.child_trade_volume,
    });

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
    fn a_region_set_on_expansion_gains_a_founding_bonus() {
        let data = GameData::load().unwrap();
        let region = named("aldervale");
        let threshold = data.balance.civilization.apply_threshold;
        let frontier = &data.balance.frontier;
        let expansion = data
            .agendas
            .iter()
            .position(|a| a.id == "expansion")
            .unwrap();

        // Boost Expansion to this region's prevailing course.
        let mut entry =
            crate::world::RegionAgendas::new("aldervale".to_owned(), data.agendas.len());
        entry.boosts[expansion] = 500.0;
        let civ = vec![entry];
        let bonus = expansion_bonus(&region, &civ, &data.agendas, threshold, frontier);
        assert!(
            (bonus - frontier.expansion_found_chance).abs() < f32::EPSILON,
            "an expansion-minded region should gain exactly the founding bonus"
        );

        // A region with no civilization entry gains nothing.
        assert_eq!(
            expansion_bonus(&region, &[], &data.agendas, threshold, frontier),
            0.0
        );
    }

    #[test]
    fn resident_rangers_lend_a_founding_bonus() {
        use crate::data::{HeroRole, HeroSeed};
        let data = GameData::load().unwrap();
        let frontier = &data.balance.frontier;
        let ranger = |id: &str, region: &str, alive: bool| {
            let mut h = Hero::from_seed(&HeroSeed {
                id: id.to_owned(),
                name: id.to_owned(),
                role: HeroRole::Ranger,
                region_id: region.to_owned(),
                level: 10,
                age: 30,
            });
            h.is_alive = alive;
            h
        };
        let mut scout = ranger("scout2", "home", true);
        scout.role = HeroRole::Warrior; // a warrior is no pathfinder

        let heroes = vec![
            ranger("pathfinder", "home", true), // counts
            ranger("second", "home", true),     // counts
            ranger("distant", "away", true),    // wrong region
            ranger("fallen", "home", false),    // dead
            scout,                              // wrong role
        ];
        // Two living rangers at home -> exactly two steps of the bonus.
        assert!(
            (ranger_bonus("home", &heroes, frontier) - 2.0 * frontier.ranger_found_chance).abs()
                < f32::EPSILON,
            "two resident rangers should lend exactly two steps of the founding bonus"
        );
        assert_eq!(
            ranger_bonus("elsewhere", &heroes, frontier),
            0.0,
            "a land with no rangers gains nothing"
        );
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
