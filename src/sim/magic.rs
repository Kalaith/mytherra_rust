//! Per-tick magic research (GDD 5.6): every path advances on the world's arcane
//! affinity, matures through thresholds, and — once emerging/known — passively
//! reshapes every region. Deterministic: no RNG.

use crate::data::strings::ChronicleText;
use crate::data::{fill, MagicBalance, MagicStat, RegionBalance};
use crate::world::{Chronicle, EventKind, MagicPath, MagicState, Region};

/// Advance every research path by one tick and apply mature paths' effects.
#[allow(clippy::too_many_arguments)]
pub fn tick_magic(
    paths: &mut [MagicPath],
    regions: &mut [Region],
    balance: &MagicBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    let avg_magic = average_magic(regions);

    for path in paths.iter_mut() {
        path.progress =
            (path.progress + balance.progress_per_tick + avg_magic * balance.magic_affinity_coeff)
                .min(balance.stat_cap);
        path.evidence = (path.evidence + balance.evidence_per_tick).min(balance.stat_cap);
        path.recompute_state(balance);

        if path.state == MagicState::Known && !path.announced_known {
            path.announced_known = true;
            chronicle.push(
                year,
                EventKind::System,
                fill(&text.magic_known, &[("path", path.name.clone())]),
            );
        }

        let scale = path.effect_scale(balance);
        if scale > 0.0 {
            let (dp, dc, dd, dm) = stat_deltas(path.effect_stat, path.effect_per_tick * scale);
            for region in regions.iter_mut() {
                region.apply_deltas(dp, dc, dd, dm, region_balance);
            }
        }
    }
}

fn average_magic(regions: &[Region]) -> f32 {
    if regions.is_empty() {
        return 0.0;
    }
    regions.iter().map(|r| r.magic_affinity).sum::<f32>() / regions.len() as f32
}

/// Map a magic stat + amount onto (prosperity, chaos, danger, magic) deltas.
fn stat_deltas(stat: MagicStat, amount: f32) -> (f32, f32, f32, f32) {
    match stat {
        MagicStat::Prosperity => (amount, 0.0, 0.0, 0.0),
        MagicStat::Chaos => (0.0, amount, 0.0, 0.0),
        MagicStat::Danger => (0.0, 0.0, amount, 0.0),
        MagicStat::Magic => (0.0, 0.0, 0.0, amount),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::world::WorldState;

    #[test]
    fn research_paths_mature_over_time() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        for _ in 0..80 {
            tick_magic(
                &mut world.magic_paths,
                &mut world.regions,
                &data.balance.magic,
                &data.balance.region,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
        }
        assert!(world
            .magic_paths
            .iter()
            .any(|p| p.state != MagicState::Dormant));
    }
}
