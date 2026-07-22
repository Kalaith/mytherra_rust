//! Per-tick magic research (GDD 5.6): every path advances on the world's arcane
//! affinity, matures through thresholds, and — once emerging/known — passively
//! reshapes every region; a fully Known path reaches living heroes too, letting
//! legend grow in attuned lands. Deterministic: no RNG.

use crate::data::strings::ChronicleText;
use crate::data::{fill, ArtifactFocus, Culture, HeroRole, MagicBalance, MagicStat, RegionBalance};
use crate::world::{Artifact, Chronicle, EventKind, Hero, Landmark, MagicPath, MagicState, Region};

/// Advance every research path by one tick and apply mature paths' effects.
#[allow(clippy::too_many_arguments)]
pub fn tick_magic(
    paths: &mut [MagicPath],
    regions: &mut [Region],
    heroes: &mut [Hero],
    artifacts: &[Artifact],
    landmarks: &[Landmark],
    balance: &MagicBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    let avg_magic = average_magic(regions);

    // Evidence of the arcane builds fastest where minds study it: living scholars
    // and mages hasten every path's maturation, so a learned age masters magic
    // sooner than an unlettered one (GDD 5.6 <-> 5.4). Counted once, applied to
    // all paths.
    let learned = heroes
        .iter()
        .filter(|h| h.is_alive && matches!(h.role, HeroRole::Scholar | HeroRole::Mage))
        .count();
    let scholar_evidence = learned as f32 * balance.evidence_per_scholar;

    // A relic of knowledge is itself a font of understanding: every Knowledge-
    // focus artifact hastens research by its power, so the Artifacts tool feeds
    // the Magic tool (GDD 5.6). Distinct from a relic's affinity nudge — this is
    // insight into the arcane (evidence), not the ambient magic of the land.
    let relic_evidence = artifacts
        .iter()
        .filter(|a| a.focus == ArtifactFocus::Knowledge)
        .map(|a| a.power as f32 * balance.evidence_per_knowledge_relic)
        .sum::<f32>();

    // The great libraries and arcane towers are the houses of the world's
    // learning: every scholarly or mystical wonder hastens research by its
    // cultural weight — its influence times its storied stature — so a land of
    // such wonders masters magic sooner, an ancient one more than a new (GDD 5.6
    // <-> 5.2).
    let landmark_evidence = landmarks
        .iter()
        .filter(|l| matches!(l.culture, Culture::Scholarly | Culture::Mystical))
        .map(|l| l.influence * l.stature * balance.evidence_per_learned_landmark)
        .sum::<f32>();

    for path in paths.iter_mut() {
        path.progress =
            (path.progress + balance.progress_per_tick + avg_magic * balance.magic_affinity_coeff)
                .min(balance.stat_cap);
        path.evidence = (path.evidence
            + balance.evidence_per_tick
            + scholar_evidence
            + relic_evidence
            + landmark_evidence)
            .min(balance.stat_cap);
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
            &world.artifacts,
            &world.landmarks,
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
            &world.artifacts,
            &world.landmarks,
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
    fn scholars_and_mages_hasten_the_discovery_of_magic() {
        use crate::data::HeroRole;
        let data = GameData::load().unwrap();
        // Evidence a fresh Dormant path accrues in one tick, given a hero roster.
        let evidence_after = |roles: &[HeroRole]| {
            let mut world = WorldState::new(&data);
            let region_id = world.regions[0].id.clone();
            world.heroes = roles
                .iter()
                .enumerate()
                .map(|(i, &role)| Hero {
                    id: format!("h{i}"),
                    name: format!("h{i}"),
                    role,
                    region_id: region_id.clone(),
                    level: 1,
                    age: 20,
                    is_alive: true,
                    renown: 0.0,
                })
                .collect();
            world.magic_paths.clear();
            world.magic_paths.push(MagicPath {
                id: "p".to_owned(),
                name: "Test Art".to_owned(),
                description: String::new(),
                effect_stat: MagicStat::Magic,
                effect_per_tick: 0.0,
                progress: 0.0,
                evidence: 0.0,
                state: MagicState::Dormant,
                announced_known: false,
            });
            tick_magic(
                &mut world.magic_paths,
                &mut world.regions,
                &mut world.heroes,
                &world.artifacts,
                &world.landmarks,
                &data.balance.magic,
                &data.balance.region,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
            world.magic_paths[0].evidence
        };

        let learned = evidence_after(&[HeroRole::Scholar, HeroRole::Mage, HeroRole::Scholar]);
        let unlettered = evidence_after(&[HeroRole::Warrior, HeroRole::Ranger]);
        assert!(
            learned > unlettered,
            "a learned society should uncover magic faster ({learned} vs {unlettered})"
        );
    }

    #[test]
    fn a_knowledge_relic_hastens_the_understanding_of_magic() {
        use crate::data::ArtifactFocus;
        let data = GameData::load().unwrap();
        // Evidence a fresh Dormant path accrues in one tick, given the relics
        // present. Heroes cleared so only the relic contribution varies.
        let evidence_after = |relics: Vec<Artifact>| {
            let mut world = WorldState::new(&data);
            world.heroes.clear();
            world.artifacts = relics;
            world.magic_paths.clear();
            world.magic_paths.push(MagicPath {
                id: "p".to_owned(),
                name: "Test Art".to_owned(),
                description: String::new(),
                effect_stat: MagicStat::Magic,
                effect_per_tick: 0.0,
                progress: 0.0,
                evidence: 0.0,
                state: MagicState::Dormant,
                announced_known: false,
            });
            tick_magic(
                &mut world.magic_paths,
                &mut world.regions,
                &mut world.heroes,
                &world.artifacts,
                &world.landmarks,
                &data.balance.magic,
                &data.balance.region,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
            world.magic_paths[0].evidence
        };

        let relic = |focus: ArtifactFocus| Artifact {
            id: "relic".to_owned(),
            name: "Test Relic".to_owned(),
            focus,
            power: 5,
            instability: 0.0,
            region_id: "aldermoor".to_owned(),
        };
        let with_knowledge = evidence_after(vec![relic(ArtifactFocus::Knowledge)]);
        let without = evidence_after(vec![]);
        let with_war = evidence_after(vec![relic(ArtifactFocus::War)]);
        assert!(
            with_knowledge > without,
            "a Knowledge relic should hasten research ({with_knowledge} vs {without})"
        );
        assert_eq!(
            with_war, without,
            "only Knowledge-focus relics feed research, not a War relic"
        );
    }

    #[test]
    fn learned_landmarks_hasten_the_understanding_of_magic() {
        use crate::data::LandmarkSeed;
        use crate::world::Landmark;
        let data = GameData::load().unwrap();
        // Evidence a fresh Dormant path accrues in one tick, given the wonders
        // present; heroes and relics cleared so only the landmarks vary.
        let evidence_after = |landmarks: Vec<Landmark>| {
            let mut world = WorldState::new(&data);
            world.heroes.clear();
            world.artifacts.clear();
            world.landmarks = landmarks;
            world.magic_paths.clear();
            world.magic_paths.push(MagicPath {
                id: "p".to_owned(),
                name: "Test Art".to_owned(),
                description: String::new(),
                effect_stat: MagicStat::Magic,
                effect_per_tick: 0.0,
                progress: 0.0,
                evidence: 0.0,
                state: MagicState::Dormant,
                announced_known: false,
            });
            tick_magic(
                &mut world.magic_paths,
                &mut world.regions,
                &mut world.heroes,
                &world.artifacts,
                &world.landmarks,
                &data.balance.magic,
                &data.balance.region,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
            world.magic_paths[0].evidence
        };

        let wonder = |culture: Culture| {
            Landmark::from_seed(&LandmarkSeed {
                id: "w".to_owned(),
                name: "The Tower".to_owned(),
                region_id: "aldermoor".to_owned(),
                culture,
                influence: 3.0,
            })
        };
        let with_tower = evidence_after(vec![wonder(Culture::Mystical)]);
        let without = evidence_after(vec![]);
        let with_forge = evidence_after(vec![wonder(Culture::Martial)]);
        assert!(
            with_tower > without,
            "an arcane tower should hasten research ({with_tower} vs {without})"
        );
        assert_eq!(
            with_forge, without,
            "only scholarly and mystical wonders feed research, not a martial one"
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
                &mut world.heroes,
                &world.artifacts,
                &world.landmarks,
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
