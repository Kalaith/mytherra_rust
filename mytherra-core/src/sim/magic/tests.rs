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
        &world.resource_nodes,
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
        &world.resource_nodes,
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
            &world.resource_nodes,
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
fn a_learned_world_grasps_the_arcane_sooner() {
    let data = GameData::load().unwrap();
    // Evidence a fresh Dormant path accrues in one tick, given the world's
    // average lore. Heroes, relics, wonders, and springs cleared so only lore
    // varies — an unlettered world (lore at the floor) against a learned one.
    let evidence_after = |lore: f32| {
        let mut world = WorldState::new(&data);
        world.heroes.clear();
        world.artifacts.clear();
        world.landmarks.clear();
        world.resource_nodes.clear();
        for region in world.regions.iter_mut() {
            region.lore = lore;
        }
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
            &world.resource_nodes,
            &data.balance.magic,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        world.magic_paths[0].evidence
    };

    let floor = data.balance.magic.evidence_lore_floor;
    let learned = evidence_after(floor + 40.0);
    let benighted = evidence_after(floor);
    assert!(
        learned > benighted,
        "a learned world should uncover magic sooner ({learned} vs {benighted})"
    );
    // A world below the common measure of learning adds nothing, not a penalty.
    assert!((evidence_after(floor - 10.0) - benighted).abs() < 1e-4);
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
            &world.resource_nodes,
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
            &world.resource_nodes,
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
fn producing_manasprings_hasten_the_understanding_of_magic() {
    use crate::data::{ResourceStatus, ResourceType};
    use crate::world::ResourceNode;
    let data = GameData::load().unwrap();
    // Evidence a fresh Dormant path accrues in one tick, given the resource
    // nodes present; heroes, relics, and wonders cleared so only the nodes vary.
    let evidence_after = |nodes: Vec<ResourceNode>| {
        let mut world = WorldState::new(&data);
        world.heroes.clear();
        world.artifacts.clear();
        world.landmarks.clear();
        world.resource_nodes = nodes;
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
            &world.resource_nodes,
            &data.balance.magic,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        world.magic_paths[0].evidence
    };

    let node = |resource_type: ResourceType, status: ResourceStatus| ResourceNode {
        id: "n".to_owned(),
        name: "N".to_owned(),
        region_id: "aldermoor".to_owned(),
        resource_type,
        status,
    };
    let with_spring = evidence_after(vec![node(ResourceType::Manaspring, ResourceStatus::Active)]);
    let without = evidence_after(vec![]);
    let dry_spring = evidence_after(vec![node(
        ResourceType::Manaspring,
        ResourceStatus::Depleted,
    )]);
    let with_mine = evidence_after(vec![node(ResourceType::Mine, ResourceStatus::Active)]);
    assert!(
        with_spring > without,
        "a producing manaspring should hasten research ({with_spring} vs {without})"
    );
    assert_eq!(
        dry_spring, without,
        "a manaspring run dry offers nothing to research"
    );
    assert_eq!(
        with_mine, without,
        "only manasprings feed research, not a mundane mine"
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
            &world.resource_nodes,
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
