use super::*;
use crate::data::BetPredicate;
use crate::world::WorldState;

#[test]
fn the_observatory_favours_notable_heroes() {
    let data = GameData::load().unwrap();
    let b = &data.balance.betting;
    let hero = |id: &str, level: u32, renown: f32, alive: bool| Hero {
        id: id.to_owned(),
        name: id.to_owned(),
        role: crate::data::HeroRole::Warrior,
        region_id: "r".to_owned(),
        level,
        age: 30,
        is_alive: alive,
        renown,
    };
    let heroes = vec![
        hero("legend", 30, 200.0, true),
        hero("novice", 1, 0.0, true),
    ];

    // Legend weight ~= 1 + 200*renown_bias + 30*level_bias; novice ~= 1.1. The
    // renowned hero should be named the large majority of the time.
    let mut rng = SeededRng::new(42);
    let legend_picks = (0..1000)
        .filter(|_| pick_notable_hero(&heroes, b, &mut rng).unwrap().id == "legend")
        .count();
    assert!(
        legend_picks > 700,
        "the Observatory should favour the legend ({legend_picks}/1000)"
    );

    // A roster with no living hero yields nobody to speculate about.
    let dead = vec![hero("gone", 5, 10.0, false)];
    assert!(pick_notable_hero(&dead, b, &mut rng).is_none());
}

#[test]
fn the_crowd_drifts_toward_the_current_likelihood() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    world.regions[0].prosperity = 90.0;
    let region_id = world.regions[0].id.clone();

    // A "prosperity >= 50" bet on a 90-prosperity region reads near-certain,
    // but its crowd opens evenly split. Drift should pull the lean toward yes.
    world.speculations.push(SpeculationEvent {
        id: "spec-drift".to_owned(),
        bet_type_name: "Test".to_owned(),
        description: String::new(),
        predicate: BetPredicate::RegionProsperityAtLeast,
        threshold: 50.0,
        target_kind: TargetKind::Region,
        target_id: region_id,
        target_name: String::new(),
        base_odds: 2.0,
        timeframe_name: String::new(),
        timeframe_modifier: 1.0,
        created_year: 1,
        deadline_year: 100,
        created_era: 1,
        created_region_count: world.regions.len() as u32,
        origin_region_id: String::new(),
        crowd_yes: 50.0,
        crowd_no: 50.0,
        resolved: None,
    });

    let lean = |e: &SpeculationEvent| e.crowd_yes / e.crowd_total();
    let before = lean(&world.speculations[0]);
    for _ in 0..20 {
        drift_crowds(
            &mut world.speculations,
            &world.heroes,
            &world.regions,
            &world.settlements,
            0.0,
            data.balance.betting.crowd_drift,
        );
    }
    let after = lean(&world.speculations[0]);
    assert!(
        after > before,
        "the crowd should lean toward the near-certain outcome: {before} -> {after}"
    );
}

#[test]
fn replenishes_up_to_target() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut player = PlayerState::new(&data.config);
    tick_speculations(
        &mut world.speculations,
        &mut world.speculation_seq,
        &mut player,
        &world.heroes,
        &world.regions,
        &world.settlements,
        &mut world.chronicle,
        &mut world.rng,
        &data,
        world.year,
        world.era.number,
        0.0,
    );
    let active = world.speculations.iter().filter(|e| e.is_active()).count();
    assert_eq!(active, data.balance.betting.active_events);
}

#[test]
fn the_crowd_leans_toward_the_likely_outcome() {
    let data = GameData::load().unwrap();
    let b = &data.balance.betting;
    let mut rng = crate::world::WorldState::new(&data).rng;
    // With noise 0.18, a near-certain proposition (0.95) always leaves the
    // crowd backing "yes" harder than "no"; a near-impossible one (0.05) the
    // reverse — the market reads the world.
    for _ in 0..64 {
        let (yes, no) = seed_crowd(0.95, &mut rng, b);
        assert!(yes > no, "the crowd should back a likely outcome");
        let (yes, no) = seed_crowd(0.05, &mut rng, b);
        assert!(no > yes, "the crowd should shun an unlikely outcome");
    }
}

#[test]
fn events_carry_unique_ids() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let mut player = PlayerState::new(&data.config);
    for _ in 0..3 {
        tick_speculations(
            &mut world.speculations,
            &mut world.speculation_seq,
            &mut player,
            &world.heroes,
            &world.regions,
            &world.settlements,
            &mut world.chronicle,
            &mut world.rng,
            &data,
            world.year,
            world.era.number,
            0.0,
        );
    }
    let mut ids: Vec<&str> = world.speculations.iter().map(|e| e.id.as_str()).collect();
    let count = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), count, "ids must be unique");
}

fn bet(id: &str, resolved: Option<bool>) -> Bet {
    Bet {
        event_id: id.to_owned(),
        predicate: crate::data::BetPredicate::default(),
        bet_type_name: "t".to_owned(),
        target_name: "x".to_owned(),
        confidence_name: "c".to_owned(),
        stake: 10,
        potential_payout: 20,
        odds: 2.0,
        placed_year: 1,
        deadline_year: 2,
        resolved,
    }
}

#[test]
fn prune_bets_keeps_pending_and_caps_resolved() {
    // 3 pending + 5 resolved, cap 2: all pending survive, only the newest 2
    // resolved (r3, r4) remain.
    let mut bets = vec![
        bet("r0", Some(true)),
        bet("p0", None),
        bet("r1", Some(false)),
        bet("r2", Some(true)),
        bet("p1", None),
        bet("r3", Some(false)),
        bet("p2", None),
        bet("r4", Some(true)),
    ];
    prune_bets(&mut bets, 2);
    let pending = bets.iter().filter(|b| b.resolved.is_none()).count();
    let resolved: Vec<&str> = bets
        .iter()
        .filter(|b| b.resolved.is_some())
        .map(|b| b.event_id.as_str())
        .collect();
    assert_eq!(pending, 3, "every pending wager must survive");
    assert_eq!(resolved, vec!["r3", "r4"], "only the newest resolved kept");
}
