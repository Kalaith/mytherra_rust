use super::*;
use crate::world::WorldState;

#[test]
fn candidates_replenish_to_target() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    tick_myths(
        &mut world.myths,
        &mut world.myth_candidates,
        &mut world.myth_seq,
        &mut world.regions,
        &mut world.heroes,
        &mut world.rng,
        &mut world.chronicle,
        &data,
        world.year,
    );
    assert_eq!(
        world.myth_candidates.len(),
        data.balance.myth.candidate_count
    );
}

#[test]
fn myths_favour_regions_that_embody_the_theme() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    // Two regions: one drenched in magic, one barren of it.
    world.regions.truncate(2);
    world.regions[0].magic_affinity = 100.0;
    world.regions[1].magic_affinity = 0.0;
    let magical_id = world.regions[0].id.clone();

    let mut rng = SeededRng::new(7);
    let mut in_magical = 0;
    for _ in 0..300 {
        let region = pick_region_by_theme(
            &world.regions,
            MythStat::Magic,
            &data.balance.myth,
            &mut rng,
        )
        .unwrap();
        if region.id == magical_id {
            in_magical += 1;
        }
    }
    // Floor 15 vs stat 100 → ~115/130 ≈ 88% land in the magical region.
    assert!(
        in_magical > 220,
        "magic myths should overwhelmingly favour the magical region ({in_magical}/300)"
    );
}

#[test]
fn a_crested_god_is_remembered_in_myth_where_its_domain_burns() {
    // A god of danger crested to wrath seeds a Danger-themed candidate rooted
    // in the region where danger runs highest, attributed to the deity by
    // name and at full resonance (GDD 5.6 pantheon <-> myths).
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    world.myth_candidates.clear();
    world.regions.truncate(2);
    world.regions[0].danger = 20.0;
    world.regions[1].danger = 95.0;
    let dire_id = world.regions[1].id.clone();
    let dire_name = world.regions[1].name.clone();

    seed_divine_myth(
        &mut world.myth_candidates,
        &mut world.myth_seq,
        "Mordath",
        MythStat::Danger,
        &world.regions,
        &data,
    );

    assert_eq!(world.myth_candidates.len(), 1);
    let seeded = &world.myth_candidates[0]; // inserted at the front
    assert_eq!(seeded.stat, MythStat::Danger);
    assert_eq!(
        seeded.region_id, dire_id,
        "rooted where the domain burns brightest"
    );
    assert!(
        seeded.title.contains("Mordath") && seeded.title.contains(&dire_name),
        "the tale names the god and its land: {}",
        seeded.title
    );
    assert_eq!(
        seeded.resonance, data.balance.myth.resonance_max,
        "a divine tale rises at full resonance"
    );
}

#[test]
fn divine_myths_stop_at_the_board_ceiling() {
    // However stormy the pantheon, divine myths never flood the board past
    // its ceiling — the player's promotion queue stays legible.
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let cap = data.balance.myth.candidate_count * 2;
    for _ in 0..cap * 2 {
        seed_divine_myth(
            &mut world.myth_candidates,
            &mut world.myth_seq,
            "Mordath",
            MythStat::Danger,
            &world.regions,
            &data,
        );
    }
    assert!(
        world.myth_candidates.len() <= cap,
        "divine myths overflowed the board ({} > {cap})",
        world.myth_candidates.len()
    );
}

#[test]
fn a_slain_beast_becomes_a_valor_legend_of_the_hunt() {
    // Felling a beast seeds a Valor tale naming both hero and beast, rooted in
    // the region where it fell, at full resonance (GDD 5.2 <-> 5.6).
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    world.myth_candidates.clear();
    let region_id = world.regions[0].id.clone();
    let region_name = world.regions[0].name.clone();

    seed_beast_myth(
        &mut world.myth_candidates,
        &mut world.myth_seq,
        "Bramwell the Bold",
        "The Shadow Wyrm",
        &region_id,
        &region_name,
        &data,
    );

    assert_eq!(world.myth_candidates.len(), 1);
    let m = &world.myth_candidates[0];
    assert!(
        m.title.contains("Bramwell") && m.title.contains("Shadow Wyrm"),
        "the tale should name both hero and beast: {}",
        m.title
    );
    let legend_theme = data
        .myth_themes
        .iter()
        .find(|t| t.id == data.balance.myth.legend_theme_id)
        .unwrap();
    assert_eq!(
        m.culture, legend_theme.culture,
        "a tale of the hunt carries the Valor theme's culture"
    );
    assert_eq!(m.region_id, region_id);
    assert_eq!(m.resonance, data.balance.myth.resonance_max);
}

#[test]
fn strong_myth_echoes_and_resets_cooldown() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let region_id = world.regions[0].id.clone();
    let region_name = world.regions[0].name.clone();
    let culture_before = world.regions[0].cultural_influence;
    world.myths.push(Myth {
        id: "m".to_owned(),
        title: "The Test".to_owned(),
        theme_name: "Valor".to_owned(),
        stat: MythStat::Prosperity,
        cultural_effect: 2.0,
        stat_effect: 1.0,
        culture: crate::data::Culture::Martial,
        region_id,
        region_name,
        resonance: 90.0,
        echo_cooldown: 0,
    });
    tick_myths(
        &mut world.myths,
        &mut world.myth_candidates,
        &mut world.myth_seq,
        &mut world.regions,
        &mut world.heroes,
        &mut world.rng,
        &mut world.chronicle,
        &data,
        world.year,
    );
    assert!(world.regions[0].cultural_influence > culture_before);
    assert_eq!(
        world.myths[0].echo_cooldown,
        data.balance.myth.echo_cooldown
    );
}

#[test]
fn an_echoing_myth_inspires_the_heroes_of_its_land() {
    use crate::data::HeroRole;
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    world.regions.truncate(2);
    let home = world.regions[0].id.clone();
    let away = world.regions[1].id.clone();

    // A hero in the myth's home region, one in another, and a fallen one at
    // home — only the living local should be inspired by the echo.
    let hero = |id: &str, region: &str, alive: bool| Hero {
        id: id.to_owned(),
        name: id.to_owned(),
        role: HeroRole::Warrior,
        region_id: region.to_owned(),
        level: 3,
        age: 25,
        is_alive: alive,
        renown: 0.0,
    };
    world.heroes = vec![
        hero("local", &home, true),
        hero("distant", &away, true),
        hero("fallen", &home, false),
    ];

    // A vivid myth ready to echo this tick in the home region.
    world.myths.clear();
    world.myths.push(Myth {
        id: "m".to_owned(),
        title: "The Old Song".to_owned(),
        theme_name: "Valor".to_owned(),
        stat: MythStat::Prosperity,
        cultural_effect: 0.0,
        stat_effect: 0.0,
        culture: crate::data::Culture::Martial,
        region_id: home.clone(),
        region_name: world.regions[0].name.clone(),
        resonance: 100.0,
        echo_cooldown: 0,
    });

    tick_myths(
        &mut world.myths,
        &mut world.myth_candidates,
        &mut world.myth_seq,
        &mut world.regions,
        &mut world.heroes,
        &mut world.rng,
        &mut world.chronicle,
        &data,
        world.year,
    );

    let renown = |id: &str| world.heroes.iter().find(|h| h.id == id).unwrap().renown;
    assert!(
        renown("local") > 0.0,
        "a tale still sung should inspire the living heroes of its land"
    );
    assert_eq!(
        renown("distant"),
        0.0,
        "the tale reaches only its own region's heroes"
    );
    assert_eq!(renown("fallen"), 0.0, "the dead take no inspiration");
}

#[test]
fn a_myth_endures_longer_where_its_theme_still_thrives() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    // Two regions: one drenched in magic, one barren of it — both hosting an
    // identical magic-myth. Silence their echoes (huge cooldown) so we
    // isolate the decay path, and hold the region stats fixed.
    world.regions.truncate(2);
    world.regions[0].magic_affinity = 100.0;
    world.regions[1].magic_affinity = 0.0;
    let vivid_id = world.regions[0].id.clone();
    let barren_id = world.regions[1].id.clone();
    let make = |region_id: &str, region_name: &str| Myth {
        id: format!("m-{region_id}"),
        title: "A Tale of Magic".to_owned(),
        theme_name: "Mystery".to_owned(),
        stat: MythStat::Magic,
        cultural_effect: 0.0,
        stat_effect: 0.0,
        culture: crate::data::Culture::Mystical,
        region_id: region_id.to_owned(),
        region_name: region_name.to_owned(),
        resonance: 80.0,
        echo_cooldown: 1_000_000,
    };
    world.myths.clear();
    world.myths.push(make(&vivid_id, "Vivid"));
    world.myths.push(make(&barren_id, "Barren"));

    // Barren decays 0.5/tick (80→25 in ~110 ticks); vivid decays at 40% of
    // that, so at 150 ticks the barren tale is gone and the vivid one holds.
    for _ in 0..150 {
        // Re-pin stats each tick in case any incidental drift occurs.
        world.regions[0].magic_affinity = 100.0;
        world.regions[1].magic_affinity = 0.0;
        tick_myths(
            &mut world.myths,
            &mut world.myth_candidates,
            &mut world.myth_seq,
            &mut world.regions,
            &mut world.heroes,
            &mut world.rng,
            &mut world.chronicle,
            &data,
            world.year,
        );
    }

    let vivid = world.myths.iter().find(|m| m.region_id == vivid_id);
    let barren = world.myths.iter().find(|m| m.region_id == barren_id);
    // The barren-land tale should have been forgotten first; the vivid one
    // still lingers in memory.
    assert!(
        barren.is_none(),
        "a tale whose theme has faded from its land should be forgotten sooner"
    );
    assert!(
        vivid.is_some(),
        "a tale whose theme still runs vivid should endure longer"
    );
}

#[test]
fn a_faint_myth_fades_from_memory_and_frees_its_slot() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let floor = data.balance.myth.forgotten_floor;
    // A barren home (fit 0) so the tale gets no sustain and decays at full
    // rate — this test isolates the forgotten-floor removal, not sustain.
    world.regions[0].prosperity = 0.0;
    world.myths.clear();
    world.myths.push(Myth {
        id: "fading".to_owned(),
        title: "The Waning Tale".to_owned(),
        theme_name: "Valor".to_owned(),
        stat: MythStat::Prosperity,
        cultural_effect: 0.0,
        stat_effect: 0.0,
        culture: crate::data::Culture::Martial,
        region_id: world.regions[0].id.clone(),
        region_name: world.regions[0].name.clone(),
        resonance: floor + 0.4, // one full decay step from being forgotten
        echo_cooldown: 5,
    });

    tick_myths(
        &mut world.myths,
        &mut world.myth_candidates,
        &mut world.myth_seq,
        &mut world.regions,
        &mut world.heroes,
        &mut world.rng,
        &mut world.chronicle,
        &data,
        world.year,
    );

    assert!(
        !world.myths.iter().any(|m| m.id == "fading"),
        "a myth worn below the forgotten floor should pass out of memory"
    );
    assert!(
        world
            .chronicle
            .iter_newest()
            .any(|e| e.message.contains("The Waning Tale") && e.message.contains("fades")),
        "a myth's fading should be chronicled"
    );
}

#[test]
fn a_legend_seeds_a_full_resonance_myth_in_its_own_land() {
    let data = GameData::load().unwrap();
    let mut candidates: Vec<MythCandidate> = Vec::new();
    let mut seq = 0;
    seed_hero_legend(
        &mut candidates,
        &mut seq,
        "Brogan",
        "kharzul",
        "Kharzul",
        &data,
    );
    assert_eq!(candidates.len(), 1);
    let m = &candidates[0];
    assert!(
        m.title.contains("Brogan"),
        "the tale names its hero: {}",
        m.title
    );
    assert_eq!(
        m.region_id, "kharzul",
        "the myth belongs to the hero's land"
    );
    assert_eq!(
        m.resonance, data.balance.myth.resonance_max,
        "a legend's tale rings at full resonance"
    );
}

#[test]
fn a_saint_seeds_a_mystical_tale_of_holiness() {
    // Raising a saint seeds a mystical tale named for the saint, rooted in the
    // land that venerates them, carrying the sacrifice theme (GDD 5.1 <-> 5.6).
    let data = GameData::load().unwrap();
    let mut candidates: Vec<MythCandidate> = Vec::new();
    let mut seq = 0;
    seed_saint_myth(
        &mut candidates,
        &mut seq,
        "Saint Corvin",
        "aldermoor",
        "Aldermoor",
        &data,
    );
    assert_eq!(candidates.len(), 1);
    let m = &candidates[0];
    assert!(
        m.title.contains("Saint Corvin"),
        "the tale names its saint: {}",
        m.title
    );
    assert_eq!(m.region_id, "aldermoor");
    assert_eq!(
        m.culture,
        crate::data::Culture::Mystical,
        "a saint's tale is a mystical one, so a land of saints grows mystical in memory"
    );
    assert_eq!(m.resonance, data.balance.myth.resonance_max);
}

#[test]
fn a_saturated_board_refuses_more_legend_myths() {
    let data = GameData::load().unwrap();
    let ceiling = data.balance.myth.candidate_count * 2;
    let mut candidates: Vec<MythCandidate> = Vec::new();
    let mut seq = 0;
    // Fill past the ceiling, then confirm no further legend tale is added.
    for _ in 0..ceiling {
        seed_hero_legend(
            &mut candidates,
            &mut seq,
            "Hero",
            "kharzul",
            "Kharzul",
            &data,
        );
    }
    let saturated = candidates.len();
    seed_hero_legend(
        &mut candidates,
        &mut seq,
        "Late",
        "kharzul",
        "Kharzul",
        &data,
    );
    assert_eq!(candidates.len(), saturated, "the board can't be flooded");
}
