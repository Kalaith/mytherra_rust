//! Per-tick settlement growth (GDD 5.3): population grows on the settlement's
//! and its region's prosperity; settlement prosperity tracks its region (raised
//! by its buildings); and a thriving settlement feeds prosperity back to that
//! region (one of the region "pressure" terms §5.2 left stubbed until now).
//! Prosperous, populous settlements also raise new buildings over time (GDD 6),
//! the one settlement effect that draws on the world RNG.

use crate::data::strings::ChronicleText;
use crate::data::{
    fill, BuildingType, Culture, RegionBalance, ResourceStatus, SettlementBalance,
    SettlementNameBank, SettlementSeed,
};
use crate::world::{Building, Chronicle, EventKind, Region, ResourceNode, Settlement};
use macroquad_toolkit::data_loader::DataRegistry;
use macroquad_toolkit::math::approach;
use macroquad_toolkit::rng::SeededRng;

#[allow(clippy::too_many_arguments)]
pub fn tick_settlements(
    settlements: &mut [Settlement],
    buildings: &[Building],
    regions: &mut [Region],
    resource_nodes: &[ResourceNode],
    balance: &SettlementBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    tier_names: &[String],
    year: u32,
) {
    for settlement in settlements.iter_mut() {
        let Some(idx) = regions.iter().position(|r| r.id == settlement.region_id) else {
            continue;
        };
        let region_prosperity = regions[idx].prosperity;
        let region_chaos = regions[idx].chaos;

        // Buildings raise the settlement's prosperity equilibrium — and one whose
        // trade draws on a resource its region actually produces earns an extra
        // bonus, so a Forge over ore or a Harbor over a fishery pays off more than
        // the same building raised over barren ground (GDD 6 <-> 5.3).
        let building_bonus: f32 = buildings
            .iter()
            .filter(|b| b.settlement_id == settlement.id)
            .map(|b| {
                let synergy = b
                    .synergy_resource
                    .is_some_and(|res| region_produces(resource_nodes, &settlement.region_id, res));
                b.prosperity_bonus
                    + if synergy {
                        balance.building_synergy_bonus
                    } else {
                        0.0
                    }
            })
            .sum();
        let supporting = (region_prosperity + building_bonus).clamp(0.0, 100.0);

        // A settlement's houses of worship hallow the land around them (GDD 6 <->
        // 5.1): every Temple raises its region's divine resonance a little each
        // tick — a built path to faith beside a Cleric's tending and the player's
        // consecration, so a temple-studded land grows faithful and tithes more.
        let resonance_bonus: f32 = buildings
            .iter()
            .filter(|b| b.settlement_id == settlement.id)
            .map(|b| b.resonance_bonus)
            .sum();
        if resonance_bonus > 0.0 {
            regions[idx].add_resonance(resonance_bonus);
        }

        // A settlement's granaries lay up grain against the lean years (GDD 6 <->
        // 5.3): every Granary keeps its region's stock a little fuller each tick, a
        // built buffer against famine beside a fertile field and a hallowed harvest,
        // so a well-stored land tips into dearth less readily and breaks it sooner.
        let harvest_bonus: f32 = buildings
            .iter()
            .filter(|b| b.settlement_id == settlement.id)
            .map(|b| b.harvest_bonus)
            .sum();
        if harvest_bonus > 0.0 {
            regions[idx].add_harvest(harvest_bonus);
        }
        let target = supporting;

        // The land feeds only so many: population swells toward a capacity set by
        // its supporting prosperity, then holds, rather than compounding forever.
        let capacity = balance.capacity_per_prosperity * supporting;
        let rate = settlement.growth_rate(region_prosperity, region_chaos, balance);
        let growth = settlement.capacity_limited_growth(rate, capacity);
        let tier_before = settlement.tier(&balance.tier_thresholds);
        settlement.population = (settlement.population * (1.0 + growth)).max(0.0);
        settlement.prosperity =
            approach(settlement.prosperity, target, balance.prosperity_drift_rate)
                .clamp(0.0, 100.0);

        // A settlement crossing a size threshold is a chronicled milestone: a
        // village swelling into a town, or a city dwindling as its people leave.
        // Growth is gradual, so at most one tier is crossed per tick.
        let tier_after = settlement.tier(&balance.tier_thresholds);
        if tier_after != tier_before {
            if let Some(name) = tier_names.get(tier_after) {
                let (line, kind) = if tier_after > tier_before {
                    (&text.settlement_ascends, EventKind::Region)
                } else {
                    (&text.settlement_declines, EventKind::Region)
                };
                chronicle.push(
                    year,
                    kind,
                    fill(
                        line,
                        &[
                            ("settlement", settlement.name.clone()),
                            ("tier", name.clone()),
                            ("region", regions[idx].name.clone()),
                        ],
                    ),
                );
            }
        }

        let contribution = settlement.region_contribution(balance);
        regions[idx].apply_deltas(contribution, 0.0, 0.0, 0.0, region_balance);
    }
}

/// Remove settlements whose population has collapsed below the abandonment floor
/// (GDD 5.3), and with them the buildings they held — a town emptied by war and
/// famine finally passes from the map rather than lingering as a ghost town.
pub fn tick_settlement_abandonment(
    settlements: &mut Vec<Settlement>,
    buildings: &mut Vec<Building>,
    balance: &SettlementBalance,
    regions: &[Region],
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    let mut abandoned_ids: Vec<String> = Vec::new();
    for settlement in settlements.iter() {
        if settlement.population < balance.abandon_population {
            abandoned_ids.push(settlement.id.clone());
            let region = regions
                .iter()
                .find(|r| r.id == settlement.region_id)
                .map(|r| r.name.clone())
                .unwrap_or_else(|| settlement.region_id.clone());
            chronicle.push(
                year,
                EventKind::Region,
                fill(
                    &text.settlement_abandoned,
                    &[("settlement", settlement.name.clone()), ("region", region)],
                ),
            );
        }
    }
    if abandoned_ids.is_empty() {
        return;
    }
    settlements.retain(|s| !abandoned_ids.contains(&s.id));
    buildings.retain(|b| !abandoned_ids.contains(&b.settlement_id));
}

/// A prosperous, populous region raises a new town over time (GDD 5.3), the
/// mirror of abandonment — so a flourishing land grows fresh settlements and a
/// frontier region born townless comes to be settled. The town starts small and
/// grows through the settlement system like any other.
#[allow(clippy::too_many_arguments)]
pub fn tick_settlement_founding(
    settlements: &mut Vec<Settlement>,
    regions: &[Region],
    seq: &mut u64,
    names: &SettlementNameBank,
    balance: &SettlementBalance,
    rng: &mut SeededRng,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    for region in regions.iter() {
        if region.prosperity < balance.found_status_min
            || region.population < balance.found_min_region_pop
        {
            continue;
        }
        let town_count = settlements
            .iter()
            .filter(|s| s.region_id == region.id)
            .count();
        if town_count >= balance.found_max_per_region {
            continue;
        }
        if !rng.chance(balance.found_chance) {
            continue;
        }

        *seq += 1;
        let name = unique_settlement_name(settlements, names, rng);
        settlements.push(Settlement::from_seed(&SettlementSeed {
            id: format!("{}-town-{}", region.id, *seq),
            name: name.clone(),
            region_id: region.id.clone(),
            population: balance.found_population,
            prosperity: region.prosperity,
        }));
        chronicle.push(
            year,
            EventKind::Region,
            fill(
                &text.settlement_founded,
                &[("settlement", name), ("region", region.name.clone())],
            ),
        );
    }
}

/// A town name from the bank (prefix + suffix), unique among existing towns.
/// Deterministic given the RNG state.
fn unique_settlement_name(
    settlements: &[Settlement],
    names: &SettlementNameBank,
    rng: &mut SeededRng,
) -> String {
    if names.prefixes.is_empty() || names.suffixes.is_empty() {
        return "New Town".to_owned();
    }
    let draw = |rng: &mut SeededRng| {
        format!(
            "{}{}",
            names.prefixes[rng.below(names.prefixes.len())],
            names.suffixes[rng.below(names.suffixes.len())],
        )
    };
    // A handful of draws almost always lands a free name (hundreds of combos);
    // if the map is somehow saturated, an ordinal guarantees uniqueness.
    for _ in 0..16 {
        let candidate = draw(rng);
        if settlements.iter().all(|s| s.name != candidate) {
            return candidate;
        }
    }
    let base = draw(rng);
    (2..)
        .map(|n| format!("{base} {n}"))
        .find(|c| settlements.iter().all(|s| &s.name != c))
        .unwrap_or(base)
}

/// Prosperous, populous settlements raise new buildings over time (GDD 6). A
/// settlement holds at most one of each building type; the chosen type is drawn
/// deterministically from the world RNG (candidates sorted for determinism, as
/// the type registry is a hash map).
#[allow(clippy::too_many_arguments)]
pub fn tick_construction(
    settlements: &[Settlement],
    regions: &[Region],
    buildings: &mut Vec<Building>,
    building_types: &DataRegistry<BuildingType>,
    balance: &SettlementBalance,
    rng: &mut SeededRng,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    for settlement in settlements {
        if settlement.prosperity < balance.construction_prosperity_min
            || settlement.population < balance.construction_population_min
        {
            continue;
        }
        if !rng.chance(balance.construction_chance) {
            continue;
        }

        // Building types this settlement doesn't already have, sorted by id so
        // the RNG draw is reproducible regardless of hash-map iteration order.
        let mut candidates: Vec<&BuildingType> = building_types
            .iter()
            .map(|(_, t)| t)
            .filter(|t| {
                !buildings
                    .iter()
                    .any(|b| b.settlement_id == settlement.id && b.type_id == t.id)
            })
            .collect();
        candidates.sort_by(|a, b| a.id.cmp(&b.id));
        if candidates.is_empty() {
            continue;
        }

        // Favour a building that fits the region's dominant culture.
        let region_culture = regions
            .iter()
            .find(|r| r.id == settlement.region_id)
            .map(|r| r.culture);
        let weight = |t: &BuildingType| {
            build_weight(t.culture, region_culture, balance.culture_affinity_weight)
        };
        let total: f32 = candidates.iter().map(|t| weight(t)).sum();
        let mut roll = rng.next_f32() * total;
        let chosen = *candidates
            .iter()
            .find(|t| {
                roll -= weight(t);
                roll <= 0.0
            })
            .unwrap_or(&candidates[candidates.len() - 1]);

        buildings.push(Building {
            id: format!("{}_{}", settlement.id, chosen.id),
            name: format!("{} {}", settlement.name, chosen.name),
            settlement_id: settlement.id.clone(),
            type_id: chosen.id.clone(),
            prosperity_bonus: chosen.prosperity_bonus,
            culture: chosen.culture,
            resonance_bonus: chosen.resonance_bonus,
            harvest_bonus: chosen.harvest_bonus,
            synergy_resource: chosen.synergy_resource,
        });
        chronicle.push(
            year,
            EventKind::Region,
            fill(
                &text.settlement_built,
                &[
                    ("settlement", settlement.name.clone()),
                    ("building", chosen.name.clone()),
                ],
            ),
        );
    }
}

/// Whether a region holds a node of the given resource kind that is still
/// producing (not run dry), so a building drawing on that trade has raw material
/// at hand (GDD 6 <-> 5.3).
fn region_produces(
    nodes: &[ResourceNode],
    region_id: &str,
    resource: crate::data::ResourceType,
) -> bool {
    nodes.iter().any(|n| {
        n.region_id == region_id
            && n.resource_type == resource
            && n.status != ResourceStatus::Depleted
    })
}

/// Selection weight for a building type given the region's dominant culture: a
/// match is boosted by `affinity`, everything else stays at the 1.0 baseline.
fn build_weight(
    building_culture: Option<Culture>,
    region_culture: Option<Culture>,
    affinity: f32,
) -> f32 {
    match (building_culture, region_culture) {
        (Some(b), Some(r)) if b == r => 1.0 + affinity,
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests;
