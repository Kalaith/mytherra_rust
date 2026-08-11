//! Per-tick pantheon behaviour (GDD 5.6): each deity's pressure drifts toward a
//! baseline, and a roused deity presses its domain upon every region scaled by
//! its pressure tier. Deterministic: no RNG.

use crate::data::{PantheonBalance, PantheonStat, RegionBalance};
use crate::world::{PantheonDeity, Region};
use macroquad_toolkit::math::approach;

/// Advance every deity by one tick and apply their domain pressure.
pub fn tick_pantheon(
    deities: &mut [PantheonDeity],
    regions: &mut [Region],
    balance: &PantheonBalance,
    region_balance: &RegionBalance,
) {
    // Snapshot every deity's pressure at tick start so the ally/rival coupling is
    // order-independent: each deity reacts to the others as they stood this tick,
    // not to whichever neighbours the loop happened to update first.
    let snapshot: Vec<(String, f32)> = deities.iter().map(|d| (d.id.clone(), d.pressure)).collect();
    let pressure_of = |id: &str| snapshot.iter().find(|(sid, _)| sid == id).map(|(_, p)| *p);

    // The gods are more present in a devout age: the world's average faith rouses
    // the whole pantheon above its resting baseline, or lets it sleep in a
    // faithless one (GDD 5.6 <-> 5.1). Read once — it lifts every deity alike.
    let avg_resonance = if regions.is_empty() {
        50.0
    } else {
        regions.iter().map(|r| r.divine_resonance).sum::<f32>() / regions.len() as f32
    };
    let faith_arousal = (avg_resonance - 50.0) * balance.faith_response;

    for deity in deities.iter_mut() {
        deity.cooldown = (deity.cooldown - 1).max(0);
        // A deity stirs toward a baseline shifted by how ascendant its domain is
        // across the world, so the state of the world rouses the gods.
        let domain = domain_average(regions, deity.effect_stat);
        // The diamond pulls too: a rival's agitation above the resting baseline
        // provokes, an ally's pressure draws toward solidarity.
        let rival = pressure_of(&deity.rival_id).unwrap_or(balance.drift_target);
        let ally = pressure_of(&deity.ally_id).unwrap_or(deity.pressure);
        let target = (balance.drift_target
            + (domain - 50.0) * balance.domain_response
            + faith_arousal
            + (rival - balance.drift_target) * balance.rival_coupling
            + (ally - deity.pressure) * balance.ally_coupling)
            .clamp(0.0, 100.0);
        deity.pressure = approach(deity.pressure, target, balance.drift_rate);

        let scale = deity.tier_multiplier(balance);
        if scale > 0.0 {
            for region in regions.iter_mut() {
                // The gods reshape the faithful lands more than the faithless
                // ones: a deity's pressure lands scaled by the region's divine
                // resonance — the same receptiveness the player's own nudges obey
                // (GDD 5.6 <-> 5.2), so one rule governs how divine will takes
                // hold, whoever wields it.
                let resonance = region.effect_multiplier(region_balance);
                let (dp, dc, dd, dm) =
                    stat_deltas(deity.effect_stat, deity.effect_amount * scale * resonance);
                region.apply_deltas(dp, dc, dd, dm, region_balance);
            }
        }
    }
}

/// Names of deities that have crested into the top pressure tier this tick but
/// hadn't before — a god roused to the height of its wrath, worth chronicling.
/// `before` is each deity's tier at tick start (index-aligned with `deities`).
pub fn deities_cresting(
    before: &[usize],
    deities: &[PantheonDeity],
    balance: &PantheonBalance,
) -> Vec<String> {
    let top = balance.tiers.len();
    deities
        .iter()
        .enumerate()
        .filter(|(i, d)| d.tier(balance) >= top && before.get(*i).copied().unwrap_or(0) < top)
        .map(|(_, d)| d.name.clone())
        .collect()
}

/// The world's average value of the stat a deity holds domain over.
fn domain_average(regions: &[Region], stat: PantheonStat) -> f32 {
    if regions.is_empty() {
        return 50.0;
    }
    let sum: f32 = regions
        .iter()
        .map(|r| match stat {
            PantheonStat::Prosperity => r.prosperity,
            PantheonStat::Chaos => r.chaos,
            PantheonStat::Danger => r.danger,
            PantheonStat::Magic => r.magic_affinity,
        })
        .sum();
    sum / regions.len() as f32
}

/// Map a pantheon stat + amount onto (prosperity, chaos, danger, magic) deltas.
fn stat_deltas(stat: PantheonStat, amount: f32) -> (f32, f32, f32, f32) {
    match stat {
        PantheonStat::Prosperity => (amount, 0.0, 0.0, 0.0),
        PantheonStat::Chaos => (0.0, amount, 0.0, 0.0),
        PantheonStat::Danger => (0.0, 0.0, amount, 0.0),
        PantheonStat::Magic => (0.0, 0.0, 0.0, amount),
    }
}

#[cfg(test)]
mod tests;
