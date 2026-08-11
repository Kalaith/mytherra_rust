use super::*;
use crate::world::WorldState;

#[test]
fn an_heir_is_named_given_and_surname_from_the_bank() {
    // A hero born during play gets a proper "Given Surname" drawn from the
    // hero name bank, so heirs read like the seeded roster (GDD 5.4).
    let data = GameData::load().unwrap();
    let mut rng = macroquad_toolkit::rng::SeededRng::new(3);
    let name = descendant_name(&data.hero_names, &mut rng);
    let parts: Vec<&str> = name.split(' ').collect();
    assert_eq!(
        parts.len(),
        2,
        "an heir's name is a given name and a surname: {name}"
    );
    assert!(
        data.hero_names.first_names.iter().any(|f| f == parts[0]),
        "the given name comes from the bank: {}",
        parts[0]
    );
    assert!(
        data.hero_names.surnames.iter().any(|s| s == parts[1]),
        "the surname comes from the bank: {}",
        parts[1]
    );
}

#[test]
fn breaking_pressure_forces_a_transition() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut player = PlayerState::new(&data.config);
    // Drive every region to maximal danger/chaos so pressure breaks.
    for region in &mut world.regions {
        region.danger = 100.0;
        region.chaos = 100.0;
        region.prosperity = 0.0;
        region.refresh_status(&data.balance.region);
    }
    let era_before = world.era.number;
    tick_era(&mut world, &mut player, &data);
    assert!(world.era.number > era_before);
    assert_eq!(world.era_history.len(), 1);

    // The closed age remembers its toll: at least one heir always rises to
    // meet the new age (GDD 5.7).
    let record = world.era_history.last().unwrap();
    assert!(
        record.heroes_risen >= 1,
        "a transition must rouse at least one heir"
    );
    assert!(
        record.heroes_lost <= world.heroes.len() as u32,
        "the fallen can't exceed the roster"
    );
}

#[test]
fn the_turning_of_an_age_sweeps_away_plague_and_beast() {
    // A plague and a beast that stalked the closing age do not outlive it: the
    // transition wipes them as it wipes the skies, and marks the sweep in the
    // chronicle (GDD 5.7 <-> 5.3/5.2).
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut player = PlayerState::new(&data.config);

    world.plagues.push(crate::world::Plague {
        id: "p".to_owned(),
        name: "The Old Fever".to_owned(),
        region_id: world.regions[0].id.clone(),
        severity: 1.0,
        age: 5,
    });
    world.monsters.push(crate::world::Monster {
        id: "m".to_owned(),
        name: "The Old Wyrm".to_owned(),
        type_id: "shadow_wyrm".to_owned(),
        region_id: world.regions[0].id.clone(),
        ferocity: 2.0,
        age: 5,
        apex: false,
    });

    // Break the age.
    for region in &mut world.regions {
        region.danger = 100.0;
        region.chaos = 100.0;
        region.prosperity = 0.0;
        region.refresh_status(&data.balance.region);
    }
    let era_before = world.era.number;
    tick_era(&mut world, &mut player, &data);

    assert!(world.era.number > era_before, "the age should have turned");
    assert!(
        world.plagues.is_empty() && world.monsters.is_empty(),
        "the new age should sweep away the old plagues and beasts"
    );
    assert!(
        world
            .chronicle
            .iter_newest()
            .any(|e| e.message.contains("sweeps away")),
        "the sweep should be chronicled"
    );
}

#[test]
fn an_ages_end_tolls_the_towns() {
    // The transition's human toll reaches the settlements, not just the
    // heroes (GDD 5.7): every town loses a share of its souls.
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut player = PlayerState::new(&data.config);
    for region in &mut world.regions {
        region.danger = 100.0;
        region.chaos = 100.0;
        region.prosperity = 0.0;
        region.refresh_status(&data.balance.region);
    }
    let before: Vec<(String, f32)> = world
        .settlements
        .iter()
        .map(|s| (s.id.clone(), s.population))
        .collect();

    tick_era(&mut world, &mut player, &data);

    assert!(world.era.number > 1, "the age should have ended");
    for (id, was) in &before {
        let now = world
            .settlements
            .iter()
            .find(|s| &s.id == id)
            .map(|s| s.population)
            .expect("settlements are not removed during the transition itself");
        assert!(now < *was, "the age's end should claim souls from {id}");
    }
}

#[test]
fn a_violent_ages_end_can_raze_wonders() {
    // A raze chance of 1.0 topples every wonder as the age turns (GDD 5.7).
    let mut data = GameData::load().unwrap();
    let a = &mut data.balance.era.aftermath;
    for delta in [
        &mut a.cataclysm,
        &mut a.collapse,
        &mut a.conquest,
        &mut a.rupture,
        &mut a.divine_war,
    ] {
        delta.landmark_raze_chance = 1.0;
    }
    let mut world = WorldState::new(&data);
    let mut player = PlayerState::new(&data.config);
    assert!(!world.landmarks.is_empty(), "the seed world has wonders");
    for region in &mut world.regions {
        region.danger = 100.0;
        region.chaos = 100.0;
        region.prosperity = 0.0;
        region.refresh_status(&data.balance.region);
    }

    tick_era(&mut world, &mut player, &data);

    assert!(world.era.number > 1, "the age should have ended");
    assert!(
        world.landmarks.is_empty(),
        "a raze-1.0 age should throw down every wonder"
    );
    assert!(
        world
            .chronicle
            .iter_newest()
            .any(|e| e.message.contains("thrown down")),
        "a razed wonder should be chronicled"
    );
}

#[test]
fn a_transition_wins_a_wager_on_the_age_ending() {
    use crate::data::BetPredicate;
    use crate::world::Bet;
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut player = PlayerState::new(&data.config);
    for region in &mut world.regions {
        region.danger = 100.0;
        region.chaos = 100.0;
        region.prosperity = 0.0;
        region.refresh_status(&data.balance.region);
    }
    player.bets.push(Bet {
        event_id: "spec-1".to_owned(),
        predicate: BetPredicate::AgeEnds,
        bet_type_name: "The Turning Age".to_owned(),
        target_name: "the present age".to_owned(),
        confidence_name: String::new(),
        stake: 10,
        potential_payout: 25,
        odds: 2.0,
        placed_year: world.year,
        deadline_year: world.year + 50,
        resolved: None,
    });
    let favor_before = player.favor;

    tick_era(&mut world, &mut player, &data);

    // The age ended, so the wager wins and its payout is credited — the era
    // boundary must not force-expire it like an ordinary bet.
    assert_eq!(player.bets[0].resolved, Some(true));
    assert_eq!(player.favor, favor_before + 25);
}

#[test]
fn conquest_momentum_raises_conquest_pressure_and_decays() {
    use crate::world::compute_scores;
    let data = GameData::load().unwrap();
    let balance = &data.balance.era;
    let mut world = WorldState::new(&data);

    // Same world, scored with and without recent conquests.
    let quiet = compute_scores(
        &world.regions,
        &world.heroes,
        &world.magic_paths,
        100,
        data.config.max_favor,
        0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        balance,
    );
    let warlike = compute_scores(
        &world.regions,
        &world.heroes,
        &world.magic_paths,
        100,
        data.config.max_favor,
        0,
        50.0,
        0.0,
        0.0,
        0.0,
        0.0,
        balance,
    );
    assert!(
        warlike.conquest > quiet.conquest,
        "recent conquests should raise Conquest pressure"
    );
    assert!(
        (warlike.conquest - quiet.conquest - 50.0 * balance.conquest_momentum_weight).abs() < 0.01,
        "the momentum term should be exactly weight * momentum"
    );

    // And the momentum bleeds off over ticks.
    world.conquest_momentum = 40.0;
    let mut player = PlayerState::new(&data.config);
    tick_era(&mut world, &mut player, &data);
    assert!(world.conquest_momentum < 40.0);
}

#[test]
fn a_worlds_dominant_culture_shapes_how_its_age_ends() {
    use crate::data::Culture;
    use crate::world::compute_scores;
    let data = GameData::load().unwrap();
    let balance = &data.balance.era;
    let mut world = WorldState::new(&data);

    let scores = |w: &WorldState| {
        compute_scores(
            &w.regions,
            &w.heroes,
            &w.magic_paths,
            100,
            data.config.max_favor,
            0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            balance,
        )
    };
    // Start from a neutral baseline (neither martial nor mystical) so the
    // culture deltas are attributable purely to the swap under test.
    for r in &mut world.regions {
        r.culture = Culture::Scholarly;
    }
    let base = scores(&world);
    // Turn every region martial without touching its stats: conquest pressure
    // rises on character alone, rupture unchanged.
    for r in &mut world.regions {
        r.culture = Culture::Martial;
    }
    let martial = scores(&world);
    assert!(
        martial.conquest > base.conquest,
        "a warlike world should trend toward a Conquest age"
    );

    // A wholly mystical world instead trends toward rupture.
    for r in &mut world.regions {
        r.culture = Culture::Mystical;
    }
    let mystical = scores(&world);
    assert!(
        mystical.rupture > base.rupture,
        "a mystical world should trend toward a Magical Rupture age"
    );
    assert!(
        (mystical.rupture - base.rupture - balance.rupture_mystical_culture).abs() < 0.01,
        "a fully mystical world adds exactly the culture weight to rupture"
    );
}

#[test]
fn a_wrathful_pantheon_drives_toward_divine_war() {
    use crate::world::{compute_scores, pantheon_wrath};
    let data = GameData::load().unwrap();
    let balance = &data.balance.era;
    let mut world = WorldState::new(&data);

    // Calm gods add nothing; a fully-roused pantheon adds exactly its weight.
    let scores = |wrath: f32| {
        compute_scores(
            &world.regions,
            &world.heroes,
            &world.magic_paths,
            100,
            data.config.max_favor,
            0,
            0.0,
            0.0,
            0.0,
            0.0,
            wrath,
            balance,
        )
    };
    let calm = scores(0.0).divine_war;
    let wrathful = scores(1.0).divine_war;
    assert!(
        wrathful > calm,
        "roused gods should drive the world toward a Divine War age"
    );
    assert!(
        (wrathful - calm - balance.divinewar_pantheon).abs() < 0.01,
        "full wrath contributes exactly the pantheon weight"
    );

    // The wrath measure itself: zero at the resting baseline, positive above.
    let target = data.balance.pantheon.drift_target;
    for d in &mut world.pantheon {
        d.pressure = target;
    }
    assert_eq!(pantheon_wrath(&world.pantheon, target), 0.0);
    for d in &mut world.pantheon {
        d.pressure = 100.0;
    }
    assert!(pantheon_wrath(&world.pantheon, target) > 0.9);
}

#[test]
fn secession_momentum_raises_collapse_pressure_and_decays() {
    use crate::world::compute_scores;
    let data = GameData::load().unwrap();
    let balance = &data.balance.era;
    let mut world = WorldState::new(&data);

    let stable = compute_scores(
        &world.regions,
        &world.heroes,
        &world.magic_paths,
        100,
        data.config.max_favor,
        0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        balance,
    );
    let fracturing = compute_scores(
        &world.regions,
        &world.heroes,
        &world.magic_paths,
        100,
        data.config.max_favor,
        0,
        0.0,
        50.0,
        0.0,
        0.0,
        0.0,
        balance,
    );
    assert!(
        fracturing.collapse > stable.collapse,
        "regions fracturing from within should raise Collapse pressure"
    );
    // Secession momentum feeds Collapse, not Conquest — the two ties stay
    // distinct.
    assert!((fracturing.conquest - stable.conquest).abs() < f32::EPSILON);

    world.secession_momentum = 40.0;
    let mut player = PlayerState::new(&data.config);
    tick_era(&mut world, &mut player, &data);
    assert!(world.secession_momentum < 40.0);
}

#[test]
fn a_world_gripped_by_plague_builds_toward_collapse() {
    // A pandemic raises Collapse pressure directly, apart from the prosperity
    // the pestilence drains (GDD 5.7 <-> 5.3). Full affliction adds exactly
    // the plague weight.
    use crate::world::compute_scores;
    let data = GameData::load().unwrap();
    let balance = &data.balance.era;
    let world = WorldState::new(&data);

    let collapse = |plague_ratio: f32, famine_ratio: f32| {
        compute_scores(
            &world.regions,
            &world.heroes,
            &world.magic_paths,
            100,
            data.config.max_favor,
            0,
            0.0,
            0.0,
            plague_ratio,
            famine_ratio,
            0.0,
            balance,
        )
        .collapse
    };

    assert!(
        collapse(1.0, 0.0) > collapse(0.0, 0.0),
        "a plague-gripped world should trend toward a Collapse age"
    );
    assert!(
        (collapse(1.0, 0.0) - collapse(0.0, 0.0) - balance.collapse_plague).abs() < 0.01,
        "a wholly plagued world adds exactly the plague weight to Collapse"
    );
    // Famine is the twin of plague: a world of failed granaries drives toward
    // Collapse the same way, adding exactly its own weight (GDD 5.7 <-> 5.3).
    assert!(
        collapse(0.0, 1.0) > collapse(0.0, 0.0),
        "a famine-gripped world should trend toward a Collapse age"
    );
    assert!(
        (collapse(0.0, 1.0) - collapse(0.0, 0.0) - balance.collapse_famine).abs() < 0.01,
        "a wholly starving world adds exactly the famine weight to Collapse"
    );
}

#[test]
fn a_legend_that_falls_at_a_transition_is_chronicled() {
    // A legend taken by an age's violent end is remembered by name, not just
    // folded into the aggregate toll (GDD 5.4 <-> 5.7).
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut player = PlayerState::new(&data.config);
    for region in &mut world.regions {
        region.danger = 100.0;
        region.chaos = 100.0;
        region.prosperity = 0.0;
        region.refresh_status(&data.balance.region);
    }
    // Make the first hero an aged legend, so it certainly dies this passage.
    let legend_bar = *data.balance.hero.renown.thresholds.last().unwrap();
    world.heroes[0].renown = legend_bar + 10.0;
    world.heroes[0].age = data.balance.era.death_age;
    let legend_name = world.heroes[0].name.clone();

    tick_era(&mut world, &mut player, &data);

    assert!(
        !world
            .heroes
            .iter()
            .any(|h| h.name == legend_name && h.is_alive),
        "the aged legend should have fallen at the transition"
    );
    assert!(
        world
            .chronicle
            .iter_newest()
            .any(|e| e.message.contains(&legend_name) && e.message.contains("legend endures")),
        "a legend's fall at a transition should be chronicled by name"
    );
}

#[test]
fn calm_world_stays_in_its_era() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut player = PlayerState::new(&data.config);
    tick_era(&mut world, &mut player, &data);
    assert_eq!(world.era.number, 1);
}

#[test]
fn aftermath_reflects_each_trigger_theme() {
    use crate::data::EraTrigger;
    let a = GameData::load().unwrap().balance.era.aftermath;
    assert!(
        a.get(EraTrigger::Collapse).prosperity > 0.0,
        "a Collapse should rebuild prosperity"
    );
    assert!(
        a.get(EraTrigger::Conquest).danger > 0.0,
        "a Conquest should leave lingering danger"
    );
    assert!(
        a.get(EraTrigger::MagicalRupture).magic > 0.0,
        "a Rupture should leave arcane residue"
    );
    assert!(
        a.get(EraTrigger::DivineWar).chaos > 0.0,
        "a Divine War should leave chaos"
    );
    assert!(
        a.get(EraTrigger::Cataclysm).danger > 0.0,
        "a Cataclysm should scar the new world"
    );
}

#[test]
fn violent_ends_take_more_heroes_and_reshape_the_heirs() {
    use crate::data::EraTrigger;
    let a = GameData::load().unwrap().balance.era.aftermath;
    // A Divine War is a deadlier passage than a Collapse.
    assert!(
        a.get(EraTrigger::DivineWar).death_mult > a.get(EraTrigger::Collapse).death_mult,
        "a divine war should be deadlier than a collapse"
    );
    assert!(
        a.get(EraTrigger::Cataclysm).death_mult > 1.0,
        "a cataclysm should raise mortality above the baseline"
    );
    // A Collapse leaves a depleted world with fewer heirs; a Divine War rouses
    // more heroes to meet the new age.
    assert!(
        a.get(EraTrigger::Collapse).descendant_mult < 1.0,
        "a collapse should leave fewer heirs"
    );
    assert!(
        a.get(EraTrigger::DivineWar).descendant_mult > 1.0,
        "a divine war should rouse more heirs"
    );
}
