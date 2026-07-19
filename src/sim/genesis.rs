//! Region genesis (GDD 5.2): the world map is not fixed. Two forces reshape it:
//!
//! - **Fracture** — a region ground down by sustained chaos and danger accrues
//!   secession pressure ("strife"); once it boils over and a capable hero is
//!   present to lead the revolt, part of it breaks away as a wholly new region.
//! - **Conquest** — a strong region annexes a trade-linked neighbour that has
//!   collapsed into crisis and has no hero to defend it, merging the loser in.
//!
//! Together they are two of the three genesis paths (the third, hero-founded
//! frontiers, is still to come). The two are deliberately complementary: a
//! high-level hero in a crisis-stricken region *defends* it from conquest but
//! can instead *lead* it to secede — so the same catalyst pushes toward fracture
//! and away from being swallowed.
//!
//! Strife accrual and conquest selection are deterministic; only the breakaway's
//! name and which towns defect flow through the world RNG, so a given seed always
//! reshapes the same way.

use crate::data::strings::{ChronicleText, GenesisText};
use crate::data::{
    fill, ConquestBalance, Culture, GameData, GenesisBalance, RegionBalance, RegionSeed,
};
use crate::world::{
    Artifact, Chronicle, EventKind, Hero, Landmark, Region, RegionAgendas, ResourceNode,
    Settlement, TradeRoute, WeatherEvent, WorldState,
};
use macroquad_toolkit::rng::SeededRng;

/// Advance region genesis by one tick: age every region's strife, then let the
/// map reshape — at most one conquest and one fracture per tick.
pub fn tick_genesis(world: &mut WorldState, data: &GameData) {
    let WorldState {
        regions,
        settlements,
        resource_nodes,
        landmarks,
        artifacts,
        weather,
        heroes,
        trade_routes,
        civilization,
        region_seq,
        rng,
        chronicle,
        year,
        ..
    } = world;
    let year = *year;
    let genesis = &data.balance.genesis;
    let conquest = &data.balance.conquest;
    let region_balance = &data.balance.region;
    let gtext = &data.strings.genesis;
    let ctext = &data.strings.chronicle;

    for region in regions.iter_mut() {
        accrue_strife(region, genesis);
    }

    maybe_conquer(
        &mut Realm {
            regions,
            settlements,
            resource_nodes,
            landmarks,
            artifacts,
            weather,
            heroes,
            trade_routes,
            civilization,
        },
        conquest,
        region_balance,
        chronicle,
        ctext,
        year,
    );

    maybe_fracture(
        regions,
        settlements,
        heroes,
        civilization,
        region_seq,
        data.agendas.len(),
        rng,
        genesis,
        region_balance,
        chronicle,
        gtext,
        ctext,
        year,
    );
}

// --- Strife -----------------------------------------------------------------

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

// --- Conquest ---------------------------------------------------------------

/// Every world collection whose rows carry a `region_id` and must follow a
/// conquered region into its new owner. Bundled so conquest can reassign them
/// without an unwieldy argument list.
struct Realm<'a> {
    regions: &'a mut Vec<Region>,
    settlements: &'a mut Vec<Settlement>,
    resource_nodes: &'a mut Vec<ResourceNode>,
    landmarks: &'a mut Vec<Landmark>,
    artifacts: &'a mut Vec<Artifact>,
    weather: &'a mut Vec<WeatherEvent>,
    heroes: &'a mut Vec<Hero>,
    trade_routes: &'a mut Vec<TradeRoute>,
    civilization: &'a mut Vec<RegionAgendas>,
}

/// A region's projected military might (GDD 5.2).
fn might(region: &Region, balance: &ConquestBalance) -> f32 {
    let martial = if region.culture == Culture::Martial {
        balance.might_martial_bonus
    } else {
        0.0
    };
    region.prosperity * balance.might_prosperity
        + region.population * balance.might_population
        + region.danger * balance.might_danger
        + martial
}

/// Does a hero strong enough to hold the region against invasion live there?
fn has_defender(heroes: &[Hero], region_id: &str, balance: &ConquestBalance) -> bool {
    heroes
        .iter()
        .any(|h| h.is_alive && h.region_id == region_id && h.level >= balance.defender_min_level)
}

/// The strongest aggressor / weakest eligible target pairing, if any conquest is
/// on. Deterministic: ranked by the might gap, ties broken toward earlier
/// regions.
fn pick_conquest(
    regions: &[Region],
    heroes: &[Hero],
    trade_routes: &[TradeRoute],
    balance: &ConquestBalance,
) -> Option<(usize, usize)> {
    if regions.len() <= balance.min_regions {
        return None;
    }
    let mut best: Option<(usize, usize, f32)> = None;
    for (ai, aggressor) in regions.iter().enumerate() {
        if aggressor.status.is_crisis() || might(aggressor, balance) < balance.aggressor_min_might {
            continue;
        }
        let a_might = might(aggressor, balance);
        for (ti, target) in regions.iter().enumerate() {
            if ti == ai || !target.status.is_crisis() {
                continue;
            }
            let gap = a_might - might(target, balance);
            if gap < balance.conquest_margin || has_defender(heroes, &target.id, balance) {
                continue;
            }
            if balance.require_trade_link
                && !trade_routes
                    .iter()
                    .any(|r| r.touches(&aggressor.id) && r.touches(&target.id))
            {
                continue;
            }
            if best.is_none_or(|(_, _, g)| gap > g) {
                best = Some((ai, ti, gap));
            }
        }
    }
    best.map(|(ai, ti, _)| (ai, ti))
}

/// Merge a crisis-stricken region into a stronger neighbour: transfer its
/// people and holdings, scar the victor with the cost of war, then remove it.
fn maybe_conquer(
    realm: &mut Realm<'_>,
    balance: &ConquestBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    let Some((winner_idx, loser_idx)) =
        pick_conquest(realm.regions, realm.heroes, realm.trade_routes, balance)
    else {
        return;
    };

    let winner_id = realm.regions[winner_idx].id.clone();
    let winner_name = realm.regions[winner_idx].name.clone();
    let loser_id = realm.regions[loser_idx].id.clone();
    let loser_name = realm.regions[loser_idx].name.clone();
    let spoils = realm.regions[loser_idx].population * balance.population_transfer;

    // Reassign everything the loser owned to its conqueror.
    for s in realm.settlements.iter_mut() {
        if s.region_id == loser_id {
            s.region_id = winner_id.clone();
        }
    }
    for n in realm.resource_nodes.iter_mut() {
        if n.region_id == loser_id {
            n.region_id = winner_id.clone();
        }
    }
    for l in realm.landmarks.iter_mut() {
        if l.region_id == loser_id {
            l.region_id = winner_id.clone();
        }
    }
    for a in realm.artifacts.iter_mut() {
        if a.region_id == loser_id {
            a.region_id = winner_id.clone();
        }
    }
    for w in realm.weather.iter_mut() {
        if w.region_id == loser_id {
            w.region_id = winner_id.clone();
        }
    }
    for h in realm.heroes.iter_mut() {
        if h.region_id == loser_id {
            h.region_id = winner_id.clone();
        }
    }
    // Trade routes fold onto the winner; any that would loop back are cut.
    for route in realm.trade_routes.iter_mut() {
        if route.region_a == loser_id {
            route.region_a = winner_id.clone();
        }
        if route.region_b == loser_id {
            route.region_b = winner_id.clone();
        }
    }
    realm.trade_routes.retain(|r| r.region_a != r.region_b);

    // The victor swells with absorbed population but pays the price of war.
    let winner = &mut realm.regions[winner_idx];
    winner.population += spoils;
    winner.strife = 0.0;
    winner.apply_deltas(
        balance.winner_prosperity,
        balance.winner_chaos,
        balance.winner_danger,
        0.0,
        region_balance,
    );

    realm.regions.retain(|r| r.id != loser_id);
    realm.civilization.retain(|c| c.region_id != loser_id);

    chronicle.push(
        year,
        EventKind::Region,
        fill(
            &text.region_conquest,
            &[("winner", winner_name), ("loser", loser_name)],
        ),
    );
}

// --- Fracture ---------------------------------------------------------------

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

/// Split a region in two if one has boiled over and found a leader: spawn the
/// breakaway, vent the parent's pressure, move the founder and any defecting
/// towns, and chronicle the schism.
#[allow(clippy::too_many_arguments)]
fn maybe_fracture(
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
    let Some(parent_idx) = pick_fracture(regions, balance) else {
        return;
    };
    let Some(founder_idx) = pick_founder(heroes, &regions[parent_idx].id, balance) else {
        // Turmoil without a leader: pressure keeps building, no region is born.
        return;
    };

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

    #[test]
    fn sustained_turmoil_fractures_a_region() {
        let data = GameData::load().unwrap();
        let mut world = primed_world(&data);
        let start = world.regions.len();

        // Keep the region turbulent; strife should cross the threshold and split.
        // The planted level-20 hero also shields it from conquest, so fracture —
        // not annexation — is the outcome.
        let mut fractured = false;
        for _ in 0..200 {
            world.regions[0].chaos = 95.0;
            world.regions[0].danger = 95.0;
            world.regions[0].refresh_status(&data.balance.region);
            tick_genesis(&mut world, &data);
            if world.regions.len() > start {
                fractured = true;
                break;
            }
        }
        assert!(fractured, "a region under sustained strife never fractured");
        let child = world.regions.last().unwrap();
        assert!(child.id.contains("-rift-"));
        assert_eq!(child.culture, Culture::Martial);
        assert!(world.civilization.iter().any(|c| c.region_id == child.id));
        assert!(world.heroes.iter().any(|h| h.region_id == child.id));
    }

    #[test]
    fn calm_region_never_reshapes() {
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
            tick_genesis(&mut world, &data);
        }
        assert_eq!(world.regions.len(), start, "a calm world reshaped itself");
    }

    #[test]
    fn turmoil_without_a_leader_only_builds_pressure() {
        let data = GameData::load().unwrap();
        let mut world = primed_world(&data);
        // Strip every hero of the level that would lead a revolt OR defend a
        // region — so the strife-ridden region can neither secede nor be saved.
        // Depress the would-be aggressors too, so conquest cannot fire and steal
        // the assertion: we want to prove pressure builds with no genesis event.
        for hero in &mut world.heroes {
            hero.level = 1;
        }
        for region in world.regions.iter_mut().skip(1) {
            region.prosperity = 10.0;
            region.population = 100.0;
        }
        let start = world.regions.len();
        for _ in 0..200 {
            world.regions[0].chaos = 95.0;
            world.regions[0].danger = 95.0;
            world.regions[0].refresh_status(&data.balance.region);
            tick_genesis(&mut world, &data);
        }
        assert_eq!(
            world.regions.len(),
            start,
            "leaderless region still reshaped"
        );
        assert!(
            world.regions[0].strife >= data.balance.genesis.fracture_threshold,
            "pressure should have kept building without a founder"
        );
        assert!(world.regions[0].status.is_crisis());
    }

    #[test]
    fn a_strong_region_conquers_a_defenceless_neighbour() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let start = world.regions.len();

        // aldermoor (idx 0) is trade-linked to kharzul (idx 1).
        let loser_id = world.regions[0].id.clone();
        let winner_id = world.regions[1].id.clone();

        // Make the winner a dominant, stable power.
        let winner = &mut world.regions[1];
        winner.prosperity = 90.0;
        winner.population = 40000.0;
        winner.chaos = 20.0;
        winner.danger = 20.0;
        winner.refresh_status(&data.balance.region);

        // Collapse the loser into a defenceless crisis.
        let loser = &mut world.regions[0];
        loser.prosperity = 8.0;
        loser.chaos = 90.0;
        loser.danger = 90.0;
        loser.population = 3000.0;
        loser.refresh_status(&data.balance.region);
        assert!(world.regions[0].status.is_crisis());
        // No hero can defend the loser.
        for hero in &mut world.heroes {
            if hero.region_id == loser_id {
                hero.level = 1;
            }
        }

        tick_genesis(&mut world, &data);

        assert_eq!(world.regions.len(), start - 1, "no region was conquered");
        assert!(
            !world.regions.iter().any(|r| r.id == loser_id),
            "the conquered region still exists"
        );
        // Its settlements now answer to the victor.
        assert!(
            world
                .settlements
                .iter()
                .filter(|s| s.region_id == winner_id)
                .count()
                >= 1
        );
        // And the schism reached the chronicle.
        assert!(world
            .chronicle
            .iter_newest()
            .any(|e| e.message.contains("absorbs it whole")));
    }

    #[test]
    fn a_defended_region_resists_conquest() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let start = world.regions.len();
        let loser_id = world.regions[0].id.clone();

        let winner = &mut world.regions[1];
        winner.prosperity = 90.0;
        winner.population = 40000.0;
        winner.chaos = 20.0;
        winner.danger = 20.0;
        winner.refresh_status(&data.balance.region);

        let loser = &mut world.regions[0];
        loser.prosperity = 8.0;
        loser.chaos = 90.0;
        loser.danger = 90.0;
        loser.population = 3000.0;
        loser.refresh_status(&data.balance.region);

        // A lone champion holds the line.
        world.heroes[0].region_id = loser_id.clone();
        world.heroes[0].level = 30;
        world.heroes[0].is_alive = true;

        for _ in 0..5 {
            tick_genesis(&mut world, &data);
        }
        assert_eq!(
            world.regions.len(),
            start,
            "a defended region was conquered anyway"
        );
        assert!(world.regions.iter().any(|r| r.id == loser_id));
    }
}
