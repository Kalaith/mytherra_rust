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
            let amount = path.effect_per_tick * scale;
            for region in regions.iter_mut() {
                // Magic bites deepest where the arcane runs strong.
                let attunement =
                    balance.affinity_base + region.magic_affinity * balance.affinity_coeff;
                let (dp, dc, dd, dm) = stat_deltas(path.effect_stat, amount * attunement);
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
    fn magic_manifests_strongest_where_affinity_is_high() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        world.regions.truncate(2);
        world.regions[0].magic_affinity = 100.0;
        world.regions[0].prosperity = 50.0;
        world.regions[1].magic_affinity = 0.0;
        world.regions[1].prosperity = 50.0;

        // A single Known path that lifts prosperity.
        world.magic_paths.clear();
        world.magic_paths.push(MagicPath {
            id: "p".to_owned(),
            name: "Test Art".to_owned(),
            description: String::new(),
            effect_stat: MagicStat::Prosperity,
            effect_per_tick: 1.0,
            progress: data.balance.magic.known_progress,
            evidence: data.balance.magic.known_evidence,
            state: MagicState::Known,
            announced_known: true,
        });

        tick_magic(
            &mut world.magic_paths,
            &mut world.regions,
            &data.balance.magic,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );

        let attuned_gain = world.regions[0].prosperity - 50.0;
        let barren_gain = world.regions[1].prosperity - 50.0;
        assert!(
            attuned_gain > barren_gain,
            "magic should manifest more strongly in the attuned region ({attuned_gain} vs {barren_gain})"
        );
    }

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
