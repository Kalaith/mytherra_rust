//! Per-tick region drift (GDD 5.2). Deterministic: no RNG, pure state-in /
//! state-out, so the same world always evolves the same way.

use crate::data::RegionBalance;
use crate::world::Region;
use macroquad_toolkit::math::approach;

/// Advance a single region by one world tick.
///
/// Prosperity mean-reverts toward an equilibrium set by chaos and danger (a
/// turbulent region can't be prosperous), so the world settles dynamically
/// rather than climbing to the ceiling once every other system stacks its
/// positive contributions on top. Chaos, danger and magic relax toward calmer
/// baselines when left untended. All drift values are tuned in `balance.json`.
pub fn tick_region(region: &mut Region, balance: &RegionBalance) {
    let d = &balance.drift;
    let prosperity_target = (d.prosperity_target_base
        - region.chaos * d.prosperity_chaos_weight
        - region.danger * d.prosperity_danger_weight)
        .clamp(0.0, 100.0);
    region.prosperity = (region.prosperity
        + (prosperity_target - region.prosperity) * d.prosperity_reversion_rate)
        .clamp(0.0, 100.0);

    region.chaos = approach(region.chaos, d.chaos_target, d.chaos_rate);
    // A region's climate sets the danger it settles toward: a frozen waste or a
    // parched desert never grows as safe as a temperate vale (GDD 5.2).
    let danger_target =
        (d.danger_target + d.climate_danger.danger_offset(region.climate)).clamp(0.0, 100.0);
    region.danger = approach(region.danger, danger_target, d.danger_rate);
    region.magic_affinity = (region.magic_affinity
        + (d.magic_target - region.magic_affinity) * d.magic_reversion_rate)
        .clamp(0.0, 100.0);

    region.refresh_status(balance);
}

#[cfg(test)]
mod tests;
