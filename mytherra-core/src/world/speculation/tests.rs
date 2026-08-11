use super::*;
use crate::data::{ClimateType, Culture, GameData, RegionSeed};

fn region(id: &str) -> Region {
    let balance = GameData::load().unwrap().balance.region;
    Region::from_seed(
        &RegionSeed {
            id: id.to_owned(),
            name: id.to_owned(),
            climate: ClimateType::Temperate,
            culture: Culture::Martial,
            prosperity: 20.0,
            chaos: 80.0,
            danger: 80.0,
            magic_affinity: 40.0,
            population: 3000.0,
            cultural_influence: 40.0,
            divine_resonance: 50.0,
        },
        &balance,
    )
}

fn usurpation_event(target_id: &str) -> SpeculationEvent {
    SpeculationEvent {
        id: "spec-1".to_owned(),
        bet_type_name: "Usurpation".to_owned(),
        description: String::new(),
        predicate: BetPredicate::RegionConquered,
        threshold: 0.0,
        target_kind: TargetKind::Region,
        target_id: target_id.to_owned(),
        target_name: target_id.to_owned(),
        base_odds: 4.0,
        timeframe_name: "an age".to_owned(),
        timeframe_modifier: 1.0,
        created_year: 1,
        deadline_year: 50,
        created_era: 1,
        created_region_count: 4,
        origin_region_id: String::new(),
        crowd_yes: 1.0,
        crowd_no: 1.0,
        resolved: None,
    }
}

#[test]
fn usurpation_resolves_only_once_the_region_vanishes() {
    let event = usurpation_event("kharzul");
    // While the region stands, the wager is unfulfilled.
    let standing = vec![region("kharzul")];
    assert!(!event.is_satisfied(&[], &standing, &[], 1));
    // Once conquest removes it from the map, the proposition is satisfied.
    assert!(event.is_satisfied(&[], &[], &[], 1));
}

fn hero(id: &str, renown: f32, alive: bool) -> Hero {
    Hero {
        id: id.to_owned(),
        name: id.to_owned(),
        role: crate::data::HeroRole::Warrior,
        region_id: "kharzul".to_owned(),
        level: 10,
        age: 40,
        is_alive: alive,
        renown,
    }
}

#[test]
fn a_heros_death_odds_rise_with_peril_and_fall_with_might() {
    let mut event = usurpation_event("h");
    event.predicate = BetPredicate::HeroDies;
    event.target_kind = TargetKind::Hero;

    let mut safe = region("aldermoor");
    safe.danger = 0.0;
    let mut perilous = region("kharzul");
    perilous.danger = 100.0;
    let regions = vec![safe, perilous];

    let hero_in = |region_id: &str, level: u32| Hero {
        id: "h".to_owned(),
        name: "H".to_owned(),
        role: crate::data::HeroRole::Warrior,
        region_id: region_id.to_owned(),
        level,
        age: 40,
        is_alive: true,
        renown: 0.0,
    };
    let odds = |h: Hero| event.likelihood(&[h], &regions, &[], 0.0);

    assert!(
        odds(hero_in("kharzul", 10)) > odds(hero_in("aldermoor", 10)),
        "a hero in a war-torn land is likelier to die than one at peace"
    );
    assert!(
        odds(hero_in("kharzul", 1)) > odds(hero_in("kharzul", 40)),
        "a frail hero is likelier to die than a mighty one in the same peril"
    );
}

fn legend_event(target_id: &str, threshold: f32) -> SpeculationEvent {
    let mut e = usurpation_event(target_id);
    e.predicate = BetPredicate::HeroRenownAtLeast;
    e.target_kind = TargetKind::Hero;
    e.threshold = threshold;
    e
}

#[test]
fn legend_resolves_only_for_a_living_hero_past_the_renown_bar() {
    let event = legend_event("brogan", 100.0);
    // Below the bar: unfulfilled.
    assert!(!event.is_satisfied(&[hero("brogan", 60.0, true)], &[], &[], 1));
    // At/over the bar while alive: a legend.
    assert!(event.is_satisfied(&[hero("brogan", 120.0, true)], &[], &[], 1));
    // A fallen hero can never win the wager, however renowned.
    assert!(!event.is_satisfied(&[hero("brogan", 200.0, false)], &[], &[], 1));
}

#[test]
fn legend_likelihood_scales_with_renown_and_zeroes_on_death() {
    let event = legend_event("brogan", 100.0);
    let rising = event.likelihood(&[hero("brogan", 50.0, true)], &[], &[], 0.0);
    assert!((rising - 0.5).abs() < 0.01, "halfway to the bar reads ~0.5");
    assert_eq!(
        event.likelihood(&[hero("brogan", 40.0, false)], &[], &[], 0.0),
        0.0
    );
}

/// A hero at a given age, region, and vitality, for the lifespan/defection
/// wagers.
fn aged_hero(id: &str, age: u32, region_id: &str, alive: bool) -> Hero {
    Hero {
        id: id.to_owned(),
        name: id.to_owned(),
        role: crate::data::HeroRole::Warrior,
        region_id: region_id.to_owned(),
        level: 5,
        age,
        is_alive: alive,
        renown: 0.0,
    }
}

#[test]
fn a_long_life_is_won_by_reaching_the_age_even_in_death() {
    let mut event = usurpation_event("elder");
    event.predicate = BetPredicate::HeroSurvivesToAge;
    event.target_kind = TargetKind::Hero;
    event.threshold = 75.0;

    // Short of the age: unfulfilled.
    assert!(!event.is_satisfied(&[aged_hero("elder", 40, "kharzul", true)], &[], &[], 1));
    // Past the age while living: won.
    assert!(event.is_satisfied(&[aged_hero("elder", 80, "kharzul", true)], &[], &[], 1));
    // Reaching the age settles it even if the hero has since died — age freezes
    // at death, so the years lived still stand.
    assert!(event.is_satisfied(&[aged_hero("elder", 80, "kharzul", false)], &[], &[], 1));
    // But a hero who fell short never clears the bar.
    assert!(!event.is_satisfied(&[aged_hero("elder", 60, "kharzul", false)], &[], &[], 1));
}

#[test]
fn a_defection_resolves_once_the_hero_leaves_their_home() {
    let mut event = usurpation_event("wanderer");
    event.predicate = BetPredicate::HeroChangesRegion;
    event.target_kind = TargetKind::Hero;
    event.origin_region_id = "kharzul".to_owned();

    // Still home: no defection.
    assert!(!event.is_satisfied(&[aged_hero("wanderer", 30, "kharzul", true)], &[], &[], 1));
    // Settled in another land: the wager is met, dead or alive.
    assert!(event.is_satisfied(&[aged_hero("wanderer", 30, "aldermoor", true)], &[], &[], 1));
    assert!(event.is_satisfied(
        &[aged_hero("wanderer", 55, "aldermoor", false)],
        &[],
        &[],
        1
    ));

    // Without a recorded home (any other predicate's empty origin), a stray
    // HeroChangesRegion check never spuriously fires.
    event.origin_region_id = String::new();
    assert!(!event.is_satisfied(&[aged_hero("wanderer", 30, "aldermoor", true)], &[], &[], 1));
}

#[test]
fn usurpation_likelihood_reflects_vulnerability() {
    let event = usurpation_event("kharzul");
    // A weak, crisis-stricken region reads as more likely to fall than a
    // vanished one reads as certain.
    let weak = vec![region("kharzul")];
    let vulnerable = event.likelihood(&[], &weak, &[], 0.0);
    assert!(vulnerable > 0.0 && vulnerable < 1.0);
    assert_eq!(event.likelihood(&[], &[], &[], 0.0), 1.0);
}

#[test]
fn the_crowd_prices_in_a_strong_defender() {
    // The same crisis-stricken region reads as far less likely to be
    // conquered once a strong, living hero holds it — the crowd knows a
    // guarded region rarely falls (GDD 5.5 <-> 5.4). A frail nobody does not
    // move the odds, so the crowd distinguishes a real defender.
    let event = usurpation_event("kharzul");
    let weak = vec![region("kharzul")];
    let undefended = event.likelihood(&[], &weak, &[], 0.0);

    let champion = hero("guardian", 0.0, true); // level 10, home is kharzul
    let defended = event.likelihood(&[champion], &weak, &[], 0.0);
    assert!(
        defended < undefended * 0.5,
        "a guarded region should read as far less likely to fall: {defended} vs {undefended}"
    );

    let nobody = Hero {
        level: 1,
        renown: 0.0,
        ..hero("nobody", 0.0, true)
    };
    let still_open = event.likelihood(&[nobody], &weak, &[], 0.0);
    assert!(
        (still_open - undefended).abs() < 1e-6,
        "a frail hero is no shield, so the odds are unchanged"
    );
}

#[test]
fn a_renaissance_resolves_when_culture_clears_the_bar() {
    // `region()` seeds cultural_influence at 40.
    let mut event = usurpation_event("kharzul");
    event.predicate = BetPredicate::RegionCultureAtLeast;
    let regions = vec![region("kharzul")];

    event.threshold = 30.0;
    assert!(
        event.is_satisfied(&[], &regions, &[], 1),
        "40 clears a bar of 30"
    );
    event.threshold = 60.0;
    assert!(
        !event.is_satisfied(&[], &regions, &[], 1),
        "40 falls short of 60"
    );
    // Likelihood tracks the ratio to the bar.
    assert!((event.likelihood(&[], &regions, &[], 0.0) - 40.0 / 60.0).abs() < 0.01);
}

#[test]
fn hallowed_ground_resolves_when_resonance_clears_the_bar() {
    // `region()` seeds divine_resonance at 50.
    let mut event = usurpation_event("kharzul");
    event.predicate = BetPredicate::RegionResonanceAtLeast;
    let regions = vec![region("kharzul")];

    event.threshold = 40.0;
    assert!(
        event.is_satisfied(&[], &regions, &[], 1),
        "50 clears a bar of 40"
    );
    event.threshold = 80.0;
    assert!(
        !event.is_satisfied(&[], &regions, &[], 1),
        "50 falls short of 80"
    );
    // With no clerics, likelihood tracks the raw ratio to the bar.
    assert!((event.likelihood(&[], &regions, &[], 0.0) - 50.0 / 80.0).abs() < 0.01);

    // The crowd reads the devout: a resident living Cleric lends confidence
    // that the land will grow hallowed, so the odds rise above the raw ratio.
    let barren = event.likelihood(&[], &regions, &[], 0.0);
    let served = event.likelihood(&[hero_cleric("kharzul", true)], &regions, &[], 0.0);
    assert!(
        served > barren,
        "a land served by a cleric should read likelier to grow hallowed"
    );
    // A fallen cleric tends no faith, so it does not move the odds.
    let fallen = event.likelihood(&[hero_cleric("kharzul", false)], &regions, &[], 0.0);
    assert!(
        (fallen - barren).abs() < 1e-6,
        "the dead consecrate nothing"
    );
}

fn hero_cleric(region_id: &str, alive: bool) -> Hero {
    Hero {
        id: "cleric".to_owned(),
        name: "Cleric".to_owned(),
        role: HeroRole::Cleric,
        region_id: region_id.to_owned(),
        level: 5,
        age: 40,
        is_alive: alive,
        renown: 0.0,
    }
}

#[test]
fn the_turning_age_resolves_once_the_era_advances() {
    let mut event = usurpation_event("");
    event.predicate = BetPredicate::AgeEnds;
    event.target_kind = TargetKind::World;
    event.created_era = 3;
    // Still the same age: unfulfilled.
    assert!(!event.is_satisfied(&[], &[], &[], 3));
    // A new age has dawned: satisfied.
    assert!(event.is_satisfied(&[], &[], &[], 4));
    // A near-breaking age reads far likelier to turn than a calm one.
    let calm = event.likelihood(&[], &[], &[], 0.2);
    let breaking = event.likelihood(&[], &[], &[], 0.95);
    assert!(breaking > calm && calm < 0.1);
}

#[test]
fn a_new_land_resolves_once_the_region_count_grows() {
    let mut event = usurpation_event("");
    event.predicate = BetPredicate::NewRegion;
    event.target_kind = TargetKind::World;
    event.created_region_count = 4;
    // Same four regions: unfulfilled.
    let four = vec![region("a"), region("b"), region("c"), region("d")];
    assert!(!event.is_satisfied(&[], &four, &[], 1));
    // A fifth region has risen: satisfied.
    let five = vec![
        region("a"),
        region("b"),
        region("c"),
        region("d"),
        region("e"),
    ];
    assert!(event.is_satisfied(&[], &five, &[], 1));
}
