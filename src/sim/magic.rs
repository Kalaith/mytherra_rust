//! Per-tick magic research (GDD 5.6): every path advances on the world's arcane
//! affinity, matures through thresholds, and — once emerging/known — passively
//! reshapes every region; a fully Known path reaches living heroes too, letting
//! legend grow in attuned lands. Deterministic: no RNG.

use crate::data::strings::ChronicleText;
use crate::data::{fill, MagicBalance, MagicStat, RegionBalance};
use crate::world::{Chronicle, EventKind, Hero, MagicPath, MagicState, Region};

/// Advance every research path by one tick and apply mature paths' effects.
#[allow(clippy::too_many_arguments)]
pub fn tick_magic(
    paths: &mut [MagicPath],
    regions: &mut [Region],
    heroes: &mut [Hero],
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

    // Magic reaches living things, not just the land: each Known path lets legend
    // grow, granting living heroes renown scaled by their region's attunement
    // (GDD 5.6 — the deepest of the seven tools).
    let known = paths
        .iter()
        .filter(|p| p.state == MagicState::Known)
        .count();
    if known > 0 && balance.known_renown_per_tick > 0.0 {
        let base = known as f32 * balance.known_renown_per_tick;
        for hero in heroes.iter_mut().filter(|h| h.is_alive) {
            let attunement = regions
                .iter()
                .find(|r| r.id == hero.region_id)
                .map(|r| balance.affinity_base + r.magic_affinity * balance.affinity_coeff)
                .unwrap_or(balance.affinity_base);
            hero.renown += base * attunement;
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
            &mut world.heroes,
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
    fn known_magic_breeds_legends_in_attuned_lands() {
        use crate::data::HeroRole;
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        world.regions.truncate(2);
        world.regions[0].magic_affinity = 100.0;
        world.regions[1].magic_affinity = 0.0;
        let (r0, r1) = (world.regions[0].id.clone(), world.regions[1].id.clone());

        // A single Known path (no region effect) so only the hero-renown reach
        // is under test.
        world.magic_paths.clear();
        world.magic_paths.push(MagicPath {
            id: "p".to_owned(),
            name: "Test Art".to_owned(),
            description: String::new(),
            effect_stat: MagicStat::Magic,
            effect_per_tick: 0.0,
            progress: data.balance.magic.known_progress,
            evidence: data.balance.magic.known_evidence,
            state: MagicState::Known,
            announced_known: true,
        });

        let hero = |id: &str, region: &str, alive: bool| Hero {
            id: id.to_owned(),
            name: id.to_owned(),
            role: HeroRole::Mage,
            region_id: region.to_owned(),
            level: 5,
            age: 30,
            is_alive: alive,
            renown: 0.0,
        };
        world.heroes = vec![
            hero("attuned", &r0, true),
            hero("barren", &r1, true),
            hero("fallen", &r0, false),
        ];

        tick_magic(
            &mut world.magic_paths,
            &mut world.regions,
            &mut world.heroes,
            &data.balance.magic,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );

        let renown = |id: &str| world.heroes.iter().find(|h| h.id == id).unwrap().renown;
        assert!(
            renown("barren") > 0.0,
            "a Known path reaches every living hero"
        );
        assert!(
            renown("attuned") > renown("barren"),
            "legends grow faster in an arcane-attuned land"
        );
        assert_eq!(renown("fallen"), 0.0, "the dead win no new renown");
    }

    #[test]
    fn research_paths_mature_over_time() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        for _ in 0..80 {
            tick_magic(
                &mut world.magic_paths,
                &mut world.regions,
                &mut world.heroes,
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
