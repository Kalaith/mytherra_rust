//! Fracture (GDD 5.2): a region ground down by sustained chaos and danger
//! accrues secession pressure ("strife"); once it boils over and a capable hero
//! is present to lead the revolt, part of it breaks away as a wholly new region,
//! carrying off population, a share of the towns, and its founder.

use crate::data::strings::{ChronicleText, GenesisText};
use crate::data::{fill, ArtifactFocus, Culture, GenesisBalance, RegionBalance, RegionSeed};
use crate::world::{
    Artifact, Chronicle, EventKind, Hero, Region, RegionAgendas, Settlement, TradeRoute,
};
use macroquad_toolkit::rng::SeededRng;

/// Build or bleed a region's secession pressure for this tick (deterministic).
/// A calm region sheds strife faster than a turbulent one builds it, so only a
/// *sustained* crisis fractures. A Knowledge relic bound to the region bleeds
/// strife on top of that — the player's lever to quell secession by reason.
pub(super) fn accrue_strife(region: &mut Region, artifacts: &[Artifact], balance: &GenesisBalance) {
    let pressure = region.pressure();
    if pressure > balance.strife_pressure_threshold {
        let over = pressure - balance.strife_pressure_threshold;
        region.strife = (region.strife + balance.strife_gain + over * balance.strife_over_scale)
            .min(balance.strife_cap);
    } else {
        region.strife = (region.strife - balance.strife_decay).max(0.0);
    }
    let relief = knowledge_relief(&region.id, artifacts, balance);
    if relief > 0.0 {
        region.strife = (region.strife - relief).max(0.0);
    }
}

/// Strife bled from a region by Knowledge artifacts bound to it (GDD 5.6 ↔ 5.2).
fn knowledge_relief(region_id: &str, artifacts: &[Artifact], balance: &GenesisBalance) -> f32 {
    artifacts
        .iter()
        .filter(|a| a.focus == ArtifactFocus::Knowledge && a.region_id == region_id)
        .map(|a| a.power as f32 * balance.artifact_knowledge_relief)
        .sum()
}

/// The eligible region with the most strife, if any has boiled over. Ties break
/// toward the earliest region, keeping selection deterministic.
fn pick(regions: &[Region], balance: &GenesisBalance) -> Option<usize> {
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
/// towns, and chronicle the schism. At most one fracture per tick.
#[allow(clippy::too_many_arguments)]
pub(super) fn run(
    regions: &mut Vec<Region>,
    settlements: &mut [Settlement],
    heroes: &mut [Hero],
    civ: &mut Vec<RegionAgendas>,
    trade_routes: &mut Vec<TradeRoute>,
    region_seq: &mut u64,
    secession_momentum: &mut f32,
    agenda_count: usize,
    rng: &mut SeededRng,
    balance: &GenesisBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    genesis_text: &GenesisText,
    chronicle_text: &ChronicleText,
    year: u32,
) {
    let Some(parent_idx) = pick(regions, balance) else {
        return;
    };
    let Some(founder_idx) = pick_founder(heroes, &regions[parent_idx].id, balance) else {
        // Turmoil without a leader: pressure keeps building, no region is born.
        return;
    };

    *region_seq += 1;
    let seq = *region_seq;

    // Read the parent's traits before naming the child (which borrows the whole
    // region list) and before venting the parent (which borrows it mutably).
    let parent_id = regions[parent_idx].id.clone();
    let parent_name = regions[parent_idx].name.clone();
    let parent_climate = regions[parent_idx].climate;
    let parent_danger = regions[parent_idx].danger;
    let parent_magic = regions[parent_idx].magic_affinity;
    let parent_population = regions[parent_idx].population;

    let child_id = format!("{parent_id}-rift-{seq}");
    let child_name = breakaway_name(&parent_name, regions, genesis_text, rng);
    let child_population = parent_population * balance.population_split;
    let child_seed = RegionSeed {
        id: child_id.clone(),
        name: child_name.clone(),
        climate: parent_climate,
        // Born of revolt: a breakaway takes on a martial character.
        culture: Culture::Martial,
        prosperity: balance.child_prosperity,
        chaos: balance.child_chaos,
        danger: parent_danger * balance.child_danger_carry,
        magic_affinity: parent_magic,
        population: child_population,
        cultural_influence: balance.child_cultural_influence,
        divine_resonance: balance.child_resonance,
    };

    // Vent the parent: it loses the seceding population and the pressure eases.
    let parent = &mut regions[parent_idx];
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
    civ.push(RegionAgendas::new(child_id.clone(), agenda_count));

    // Even a bitter breakaway keeps a road to the land it revolted from — a
    // strained, low-volume one, but enough that the new region isn't marooned
    // from trade, and enough that its former ruler can march back down it to
    // reconquer, a trade link being conquest's precondition (GDD 5.2).
    trade_routes.push(TradeRoute {
        id: format!("route-{parent_id}-{child_id}"),
        name: format!("{child_name} Road"),
        region_a: parent_id.clone(),
        region_b: child_id,
        volume: balance.child_trade_volume,
    });

    // Feed the world's secession momentum, which drives Collapse-era pressure.
    *secession_momentum = (*secession_momentum + balance.momentum_gain).min(balance.momentum_cap);

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
fn breakaway_name(
    parent: &str,
    regions: &[Region],
    text: &GenesisText,
    rng: &mut SeededRng,
) -> String {
    let template = rng
        .choose(&text.breakaway_names)
        .cloned()
        .unwrap_or_else(|| "Free {parent}".to_owned());
    let base = fill(&template, &[("parent", parent.to_owned())]);
    super::frontier::make_unique(base, regions)
}
