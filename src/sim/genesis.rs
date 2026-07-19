//! Region genesis (GDD 5.2): the world map is not fixed. When a region is torn
//! by sustained chaos and danger, secession pressure — "strife" — accumulates.
//! Once it boils over and a capable hero is present to lead the revolt, part of
//! the region breaks away as a wholly new region, carrying off population, a
//! share of the towns, and its founder.
//!
//! This is the first of the three genesis paths (internal strife; later:
//! conquest and hero-founded frontiers). Strife accrual is deterministic; the
//! breakaway's name and which towns defect flow through the world RNG, so a
//! given seed always fractures the same way.

use crate::data::strings::{ChronicleText, GenesisText};
use crate::data::{fill, Culture, GenesisBalance, RegionBalance, RegionSeed};
use crate::world::{Chronicle, EventKind, Hero, Region, RegionAgendas, Settlement};
use macroquad_toolkit::rng::SeededRng;

/// Advance region genesis by one tick: age every region's strife, then fracture
/// at most one region that has boiled over and found a leader.
#[allow(clippy::too_many_arguments)]
pub fn tick_genesis(
    regions: &mut Vec<Region>,
    settlements: &mut [Settlement],
    heroes: &mut [Hero],
    civ: &mut Vec<RegionAgendas>,
    region_seq: &mut u64,
    agenda_count: usize,
    rng: &mut SeededRng,
    balance: &GenesisBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    genesis_text: &GenesisText,
    chronicle_text: &ChronicleText,
    year: u32,
) {
    for region in regions.iter_mut() {
        accrue_strife(region, balance);
    }

    let Some(parent_idx) = pick_fracture(regions, balance) else {
        return;
    };
    let Some(founder_idx) = pick_founder(heroes, &regions[parent_idx].id, balance) else {
        // Turmoil without a leader: pressure keeps building, no region is born.
        return;
    };

    fracture(
        regions,
        settlements,
        heroes,
        civ,
        region_seq,
        agenda_count,
        parent_idx,
        founder_idx,
        rng,
        balance,
        region_balance,
        chronicle,
        genesis_text,
        chronicle_text,
        year,
    );
}

/// Build or bleed a region's secession pressure for this tick (deterministic).
fn accrue_strife(region: &mut Region, balance: &GenesisBalance) {
    let pressure = region.pressure();
    if pressure > balance.strife_pressure_threshold {
        let over = pressure - balance.strife_pressure_threshold;
        region.strife = (region.strife + balance.strife_gain + over * balance.strife_over_scale)
            .min(balance.strife_cap);
    } else {
        region.strife = (region.strife - balance.strife_decay).max(0.0);
    }
}

/// The eligible region with the most strife, if any has boiled over. Ties break
/// toward the earliest region, keeping selection deterministic.
fn pick_fracture(regions: &[Region], balance: &GenesisBalance) -> Option<usize> {
    regions
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            r.strife >= balance.fracture_threshold && r.population >= balance.min_population
        })
        .max_by(|(_, a), (_, b)| a.strife.total_cmp(&b.strife))
        .map(|(idx, _)| idx)
}

/// The strongest living hero in the region who can lead a breakaway. Ties break
/// toward the earliest hero, keeping selection deterministic.
fn pick_founder(heroes: &[Hero], region_id: &str, balance: &GenesisBalance) -> Option<usize> {
    heroes
        .iter()
        .enumerate()
        .filter(|(_, h)| {
            h.is_alive && h.region_id == region_id && h.level >= balance.founder_min_level
        })
        .max_by_key(|(_, h)| h.level)
        .map(|(idx, _)| idx)
}

/// Split a region in two: spawn the breakaway, vent the parent's pressure, move
/// the founder and any defecting towns, and chronicle the schism.
#[allow(clippy::too_many_arguments)]
fn fracture(
    regions: &mut Vec<Region>,
    settlements: &mut [Settlement],
    heroes: &mut [Hero],
    civ: &mut Vec<RegionAgendas>,
    region_seq: &mut u64,
    agenda_count: usize,
    parent_idx: usize,
    founder_idx: usize,
    rng: &mut SeededRng,
    balance: &GenesisBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    genesis_text: &GenesisText,
    chronicle_text: &ChronicleText,
    year: u32,
) {
    *region_seq += 1;
    let seq = *region_seq;

    let parent = &mut regions[parent_idx];
    let parent_id = parent.id.clone();
    let parent_name = parent.name.clone();

    let child_id = format!("{parent_id}-rift-{seq}");
    let child_name = breakaway_name(&parent_name, genesis_text, rng);
    let child_population = parent.population * balance.population_split;
    let child_seed = RegionSeed {
        id: child_id.clone(),
        name: child_name.clone(),
        climate: parent.climate,
        // Born of revolt: a breakaway takes on a martial character.
        culture: Culture::Martial,
        prosperity: balance.child_prosperity,
        chaos: balance.child_chaos,
        danger: parent.danger * balance.child_danger_carry,
        magic_affinity: parent.magic_affinity,
        population: child_population,
        cultural_influence: balance.child_cultural_influence,
        divine_resonance: balance.child_resonance,
    };

    // Vent the parent: it loses the seceding population and the pressure eases.
    parent.population = (parent.population - child_population).max(0.0);
    parent.strife = 0.0;
    parent.apply_deltas(
        -balance.parent_prosperity_hit,
        -balance.parent_chaos_relief,
        -balance.parent_danger_relief,
        0.0,
        region_balance,
    );

    // The catalyst leads the revolt into its new home.
    let founder_name = heroes[founder_idx].name.clone();
    heroes[founder_idx].region_id = child_id.clone();

    // A share of the parent's towns throw in with the breakaway.
    for town in settlements.iter_mut() {
        if town.region_id == parent_id && rng.chance(balance.settlement_defect_chance) {
            town.region_id = child_id.clone();
        }
    }

    let child = Region::from_seed(&child_seed, region_balance);
    regions.push(child);
    civ.push(RegionAgendas::new(child_id, agenda_count));

    chronicle.push(
        year,
        EventKind::Region,
        fill(
            &chronicle_text.region_fracture,
            &[
                ("parent", parent_name),
                ("child", child_name),
                ("founder", founder_name),
            ],
        ),
    );
}

/// Choose a breakaway's name from the data-driven templates.
fn breakaway_name(parent: &str, text: &GenesisText, rng: &mut SeededRng) -> String {
    let template = rng
        .choose(&text.breakaway_names)
        .cloned()
        .unwrap_or_else(|| "Free {parent}".to_owned());
    fill(&template, &[("parent", parent.to_owned())])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::world::WorldState;

    /// Drive one region deep into turmoil and plant a capable hero there.
    fn primed_world(data: &GameData) -> WorldState {
        let mut world = WorldState::new(data);
        let region = &mut world.regions[0];
        region.chaos = 95.0;
        region.danger = 95.0;
        region.prosperity = 10.0;
        region.refresh_status(&data.balance.region);
        let region_id = region.id.clone();
        world.heroes[0].region_id = region_id;
        world.heroes[0].level = 20;
        world.heroes[0].is_alive = true;
        world
    }

    fn run_genesis(world: &mut WorldState, data: &GameData) {
        tick_genesis(
            &mut world.regions,
            &mut world.settlements,
            &mut world.heroes,
            &mut world.civilization,
            &mut world.region_seq,
            data.agendas.len(),
            &mut world.rng,
            &data.balance.genesis,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.genesis,
            &data.strings.chronicle,
            world.year,
        );
    }

    #[test]
    fn sustained_turmoil_fractures_a_region() {
        let data = GameData::load().unwrap();
        let mut world = primed_world(&data);
        let start = world.regions.len();

        // Keep the region turbulent; strife should cross the threshold and split.
        let mut fractured = false;
        for _ in 0..200 {
            world.regions[0].chaos = 95.0;
            world.regions[0].danger = 95.0;
            world.regions[0].refresh_status(&data.balance.region);
            run_genesis(&mut world, &data);
            if world.regions.len() > start {
                fractured = true;
                break;
            }
        }
        assert!(fractured, "a region under sustained strife never fractured");
        let child = world.regions.last().unwrap();
        assert!(child.id.contains("-rift-"));
        assert_eq!(child.culture, Culture::Martial);
        // The breakaway got its own civilization bookkeeping.
        assert!(world.civilization.iter().any(|c| c.region_id == child.id));
        // Its founder now lives in the new region.
        assert!(world.heroes.iter().any(|h| h.region_id == child.id));
    }

    #[test]
    fn calm_region_never_fractures() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let start = world.regions.len();
        for _ in 0..300 {
            for region in &mut world.regions {
                region.chaos = 20.0;
                region.danger = 20.0;
                region.prosperity = 70.0;
                region.refresh_status(&data.balance.region);
            }
            run_genesis(&mut world, &data);
        }
        assert_eq!(world.regions.len(), start, "a calm world spawned a region");
    }

    #[test]
    fn turmoil_without_a_leader_only_builds_pressure() {
        let data = GameData::load().unwrap();
        let mut world = primed_world(&data);
        // Remove every would-be founder from the strife-ridden region.
        for hero in &mut world.heroes {
            hero.level = 1;
        }
        let start = world.regions.len();
        for _ in 0..200 {
            world.regions[0].chaos = 95.0;
            world.regions[0].danger = 95.0;
            world.regions[0].refresh_status(&data.balance.region);
            run_genesis(&mut world, &data);
        }
        assert_eq!(world.regions.len(), start, "leaderless revolt still split");
        assert!(
            world.regions[0].strife >= data.balance.genesis.fracture_threshold,
            "pressure should have kept building without a founder"
        );
        assert!(world.regions[0].status.is_crisis());
    }
}
