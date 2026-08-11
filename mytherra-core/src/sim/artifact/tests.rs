use super::*;
use crate::data::GameData;
use crate::world::WorldState;

#[test]
fn unstable_artifact_eventually_backlashes() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let before = world.artifacts.len();
    // Run long enough that at least one relic crosses the backlash line.
    for _ in 0..60 {
        tick_artifacts(
            &mut world.artifacts,
            &mut world.regions,
            &[],
            &mut world.pending_consequences,
            &data.balance.artifact,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
    }
    assert!(world.artifacts.len() < before, "an artifact should shatter");
}

#[test]
fn a_relic_frays_faster_in_a_chaotic_land() {
    // The same relic in the same tick gains more instability where its region
    // seethes with chaos than where it is calm (GDD 5.6).
    let data = GameData::load().unwrap();
    let instability_after = |chaos: f32| {
        let mut world = WorldState::new(&data);
        world.artifacts.clear();
        world.pending_consequences.clear();
        world.regions.truncate(1);
        world.regions[0].chaos = chaos;
        world.regions[0].magic_affinity = 50.0;
        world.artifacts.push(Artifact {
            id: "relic".to_owned(),
            name: "Test Relic".to_owned(),
            focus: ArtifactFocus::Protection,
            power: 4,
            instability: 0.0,
            region_id: world.regions[0].id.clone(),
        });
        tick_artifacts(
            &mut world.artifacts,
            &mut world.regions,
            &[],
            &mut world.pending_consequences,
            &data.balance.artifact,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        // Survives one tick either way; read its accrued instability.
        world.artifacts[0].instability
    };
    assert!(
        instability_after(90.0) > instability_after(10.0),
        "a relic in a chaotic land should destabilize faster than one at peace"
    );
}

#[test]
fn a_relic_reshapes_an_attuned_land_more_strongly() {
    // The same Prosperity relic lifts an arcane-attuned region more than a
    // barren one — a relic bites deepest where magic runs strong (GDD 5.6).
    let data = GameData::load().unwrap();
    let gain = |magic_affinity: f32| {
        let mut world = WorldState::new(&data);
        world.artifacts.clear();
        world.pending_consequences.clear();
        world.regions.truncate(1);
        world.regions[0].magic_affinity = magic_affinity;
        world.regions[0].prosperity = 50.0;
        world.artifacts.push(Artifact {
            id: "relic".to_owned(),
            name: "Test Relic".to_owned(),
            focus: ArtifactFocus::Prosperity,
            power: 4,
            instability: 0.0,
            region_id: world.regions[0].id.clone(),
        });
        tick_artifacts(
            &mut world.artifacts,
            &mut world.regions,
            &[],
            &mut world.pending_consequences,
            &data.balance.artifact,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        world.regions[0].prosperity - 50.0
    };

    assert!(
        gain(100.0) > gain(0.0),
        "a relic should reshape an attuned land more strongly than a barren one"
    );
}

#[test]
fn arcane_keepers_slow_a_relics_fraying_but_never_halt_it() {
    use crate::data::HeroRole;
    use crate::world::Hero;
    let data = GameData::load().unwrap();
    let b = &data.balance.artifact;

    // Instability a relic accrues in one tick, given the heroes keeping its
    // region; the region and relic are otherwise identical.
    let growth_with = |keepers: usize| {
        let mut world = WorldState::new(&data);
        world.artifacts.clear();
        world.pending_consequences.clear();
        world.regions.truncate(1);
        world.regions[0].chaos = 30.0;
        let region_id = world.regions[0].id.clone();
        world.heroes = (0..keepers)
            .map(|i| Hero {
                id: format!("m{i}"),
                name: format!("Keeper {i}"),
                role: HeroRole::Mage,
                region_id: region_id.clone(),
                level: 4,
                age: 30,
                is_alive: true,
                renown: 0.0,
            })
            .collect();
        world.artifacts.push(Artifact {
            id: "relic".to_owned(),
            name: "Test Relic".to_owned(),
            focus: ArtifactFocus::Protection,
            power: 3,
            instability: 0.0,
            region_id,
        });
        tick_artifacts(
            &mut world.artifacts,
            &mut world.regions,
            &world.heroes,
            &mut world.pending_consequences,
            &data.balance.artifact,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        world.artifacts[0].instability
    };

    assert!(
        growth_with(2) < growth_with(0),
        "arcane keepers should slow a relic's fraying"
    );
    assert!(
        growth_with(20) >= b.min_instability_growth,
        "however many keepers, a relic still drifts toward its doom"
    );
}
