//! Per-tick resource-node behaviour (GDD 5.3): each node's status cycles
//! through a state machine driven by regional stress (chaos + danger), and its
//! output (scaled by status) feeds prosperity back to its region — the
//! "resource pressure" term region drift (5.2) left stubbed. Randomness flows
//! through the world RNG.

use crate::data::strings::ChronicleText;
use crate::data::{fill, RegionBalance, ResourceBalance, ResourceStatus, ResourceType};
use crate::world::{Chronicle, EventKind, Region, ResourceNode};
use macroquad_toolkit::rng::SeededRng;

#[allow(clippy::too_many_arguments)]
pub fn tick_resources(
    nodes: &mut [ResourceNode],
    regions: &mut [Region],
    rng: &mut SeededRng,
    balance: &ResourceBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    for node in nodes.iter_mut() {
        let Some(idx) = regions.iter().position(|r| r.id == node.region_id) else {
            continue;
        };
        let before = node.status;
        node.status = next_status(node.status, &regions[idx], rng, balance);

        // A node crossing into one of its dramatic states — a peak, a corruption,
        // or an exhaustion — is a moment worth marking; the ordinary churn between
        // the middle states stays quiet (GDD 5.3).
        if node.status != before {
            if let Some(line) = notable_line(node.status, text) {
                chronicle.push(
                    year,
                    EventKind::Region,
                    fill(
                        line,
                        &[
                            ("node", node.name.clone()),
                            ("region", regions[idx].name.clone()),
                        ],
                    ),
                );
            }
        }

        // A healthy node lifts its region; a degraded one drags it down. A
        // hazardous node poisons the land besides: a corrupted node bleeds chaos,
        // an unstable one danger (GDD 5.3).
        let output = node.output(&balance.outputs);
        let (chaos, danger) = status_hazard(node.status, balance);
        // A manaspring's yield wells up as arcane power (magic affinity) rather
        // than granary prosperity, so an arcane resource makes a mystical land and
        // a corrupted one drains it (GDD 5.3 <-> 5.6).
        let (prosperity_c, magic_c) = if node.resource_type == ResourceType::Manaspring {
            (0.0, (output - 1.0) * balance.manaspring_magic_scale)
        } else {
            ((output - 1.0) * balance.region_output_scale, 0.0)
        };
        regions[idx].apply_deltas(prosperity_c, chaos, danger, magic_c, region_balance);
    }
}

/// Occasionally open a wholly new resource node in a prospering, populous region
/// (GDD 5.3): the counterpart to settlement founding, and the way a region born
/// resource-barren — a frontier, a young breakaway — eventually develops its own
/// wealth. The node's type follows the region's culture, so the land grows
/// resources that reinforce its character (GDD 5.3 <-> 5.2). It starts Active, so
/// it adds nothing at once, only the potential to flourish. Deterministic given
/// the RNG state.
#[allow(clippy::too_many_arguments)]
pub fn tick_resource_discovery(
    nodes: &mut Vec<ResourceNode>,
    regions: &[Region],
    seq: &mut u64,
    balance: &ResourceBalance,
    rng: &mut SeededRng,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    for region in regions.iter() {
        if region.prosperity < balance.discovery_min_prosperity
            || region.population < balance.discovery_min_population
        {
            continue;
        }
        let node_count = nodes.iter().filter(|n| n.region_id == region.id).count();
        if node_count >= balance.discovery_max_per_region {
            continue;
        }
        if !rng.chance(balance.discovery_chance) {
            continue;
        }

        *seq += 1;
        let resource_type = region.culture.favored_resource();
        let name = fill(
            &text.resource_node_name,
            &[
                ("region", region.name.clone()),
                ("type", resource_type.label().to_owned()),
            ],
        );
        nodes.push(ResourceNode {
            id: format!("{}-node-{}", region.id, *seq),
            name: name.clone(),
            region_id: region.id.clone(),
            resource_type,
            status: ResourceStatus::Active,
        });
        chronicle.push(
            year,
            EventKind::Region,
            fill(
                &text.resource_discovered,
                &[
                    ("node", name),
                    ("region", region.name.clone()),
                    ("type", resource_type.label().to_owned()),
                ],
            ),
        );
    }
}

/// The chronicle line for a node entering one of its dramatic states, or `None`
/// for the ordinary middle states that aren't worth a line.
fn notable_line(status: ResourceStatus, text: &ChronicleText) -> Option<&str> {
    match status {
        ResourceStatus::Flourishing => Some(&text.resource_flourishes),
        ResourceStatus::Corrupted => Some(&text.resource_corrupts),
        ResourceStatus::Depleted => Some(&text.resource_depletes),
        _ => None,
    }
}

/// The chaos / danger a node bleeds into its region purely by its status: a
/// corrupted node spreads chaos, an unstable one danger, all others nothing.
fn status_hazard(status: ResourceStatus, balance: &ResourceBalance) -> (f32, f32) {
    match status {
        ResourceStatus::Corrupted => (balance.corrupted_chaos, 0.0),
        ResourceStatus::Unstable => (0.0, balance.unstable_danger),
        _ => (0.0, 0.0),
    }
}

/// The status state machine (GDD 5.3): regional stress pushes nodes to degrade;
/// calm regions let them recover and thrive.
fn next_status(
    current: ResourceStatus,
    region: &Region,
    rng: &mut SeededRng,
    balance: &ResourceBalance,
) -> ResourceStatus {
    use ResourceStatus::*;
    let stress = region.chaos * balance.stress_chaos + region.danger * balance.stress_danger;
    let degrade = (balance.degrade_base + stress * balance.degrade_stress).clamp(0.0, 0.9);
    let recover = balance.recover_base;
    let improve = balance.improve_base;
    let contested_region = region.chaos >= balance.contest_chaos_threshold;

    match current {
        Flourishing => {
            if rng.chance(degrade) {
                Overworked
            } else {
                Flourishing
            }
        }
        Blessed => {
            if rng.chance(improve) {
                Flourishing
            } else if rng.chance(degrade) {
                Active
            } else {
                Blessed
            }
        }
        Active => {
            if contested_region && rng.chance(degrade) {
                Contested
            } else if stress < 30.0 && rng.chance(improve) {
                Blessed
            } else if rng.chance(degrade) {
                Overworked
            } else {
                Active
            }
        }
        Overworked => {
            if rng.chance(degrade) {
                Depleted
            } else if rng.chance(recover) {
                Active
            } else {
                Overworked
            }
        }
        Contested => {
            let corrupt = balance.corrupt_base + region.danger * balance.corrupt_danger;
            if contested_region && rng.chance(corrupt) {
                Corrupted
            } else if rng.chance(recover) {
                Active
            } else {
                Contested
            }
        }
        Corrupted => {
            if rng.chance(degrade) {
                Unstable
            } else if rng.chance(recover * 0.5) {
                Contested
            } else {
                Corrupted
            }
        }
        Unstable => {
            if rng.chance(degrade) {
                Depleted
            } else if rng.chance(recover) {
                Active
            } else {
                Unstable
            }
        }
        Depleted => {
            if rng.chance(recover * 0.4) {
                Active
            } else {
                Depleted
            }
        }
    }
}

#[cfg(test)]
mod tests;
