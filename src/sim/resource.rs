//! Per-tick resource-node behaviour (GDD 5.3): each node's status cycles
//! through a state machine driven by regional stress (chaos + danger), and its
//! output (scaled by status) feeds prosperity back to its region — the
//! "resource pressure" term region drift (5.2) left stubbed. Randomness flows
//! through the world RNG.

use crate::data::{RegionBalance, ResourceBalance, ResourceStatus};
use crate::world::{Region, ResourceNode};
use macroquad_toolkit::rng::SeededRng;

pub fn tick_resources(
    nodes: &mut [ResourceNode],
    regions: &mut [Region],
    rng: &mut SeededRng,
    balance: &ResourceBalance,
    region_balance: &RegionBalance,
) {
    for node in nodes.iter_mut() {
        let Some(idx) = regions.iter().position(|r| r.id == node.region_id) else {
            continue;
        };
        node.status = next_status(node.status, &regions[idx], rng, balance);

        // A healthy node lifts its region; a degraded one drags it down. A
        // hazardous node poisons the land besides: a corrupted node bleeds chaos,
        // an unstable one danger (GDD 5.3).
        let output = node.output(&balance.outputs);
        let contribution = (output - 1.0) * balance.region_output_scale;
        let (chaos, danger) = status_hazard(node.status, balance);
        regions[idx].apply_deltas(contribution, chaos, danger, 0.0, region_balance);
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
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::world::WorldState;

    #[test]
    fn depleted_node_contributes_negatively() {
        let data = GameData::load().unwrap();
        let contribution = (data.balance.resource.outputs.depleted - 1.0)
            * data.balance.resource.region_output_scale;
        assert!(contribution < 0.0);
        let flourishing = (data.balance.resource.outputs.flourishing - 1.0)
            * data.balance.resource.region_output_scale;
        assert!(flourishing > 0.0);
    }

    #[test]
    fn only_hazardous_nodes_poison_their_region() {
        let b = GameData::load().unwrap().balance.resource;
        assert_eq!(
            status_hazard(ResourceStatus::Corrupted, &b),
            (b.corrupted_chaos, 0.0)
        );
        assert_eq!(
            status_hazard(ResourceStatus::Unstable, &b),
            (0.0, b.unstable_danger)
        );
        assert_eq!(status_hazard(ResourceStatus::Flourishing, &b), (0.0, 0.0));
        assert_eq!(status_hazard(ResourceStatus::Active, &b), (0.0, 0.0));
    }

    #[test]
    fn a_corrupted_node_bleeds_chaos_into_its_region() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        // Freeze the state machine (no degrade/recover) so the node stays
        // Corrupted this tick, isolating the hazard bleed.
        let mut balance = data.balance.resource.clone();
        balance.degrade_base = 0.0;
        balance.degrade_stress = 0.0;
        balance.recover_base = 0.0;

        world.resource_nodes.truncate(1);
        world.resource_nodes[0].status = ResourceStatus::Corrupted;
        let region_id = world.resource_nodes[0].region_id.clone();
        let ridx = world
            .regions
            .iter()
            .position(|r| r.id == region_id)
            .unwrap();
        world.regions[ridx].chaos = 30.0;
        let chaos_before = world.regions[ridx].chaos;

        tick_resources(
            &mut world.resource_nodes,
            &mut world.regions,
            &mut world.rng,
            &balance,
            &data.balance.region,
        );

        assert_eq!(world.resource_nodes[0].status, ResourceStatus::Corrupted);
        assert!(
            world.regions[ridx].chaos > chaos_before,
            "a corrupted node should spread chaos into its region"
        );
    }

    #[test]
    fn simulation_stays_deterministic() {
        let data = GameData::load().unwrap();
        let run = || {
            let mut world = WorldState::new(&data);
            for _ in 0..40 {
                tick_resources(
                    &mut world.resource_nodes,
                    &mut world.regions,
                    &mut world.rng,
                    &data.balance.resource,
                    &data.balance.region,
                );
            }
            world
                .resource_nodes
                .iter()
                .map(|n| n.status)
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }
}
