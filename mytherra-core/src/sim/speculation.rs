//! Speculation-event lifecycle (GDD 5.5): resolve due events and their bets,
//! then generate fresh events to keep the Observatory stocked. Event generation
//! uses the world RNG; bet payouts were locked at placement, so resolution here
//! is a pure win/lose credit.

use crate::data::{fill, BettingBalance, GameData, TargetKind};
#[cfg(test)]
use crate::world::PlayerState;
use crate::world::{Bet, Chronicle, EventKind, Hero, Region, Settlement, SpeculationEvent};
use macroquad_toolkit::rng::SeededRng;

/// Resolve, settle, replenish, and prune speculation events for one tick — the
/// single-player composition the speculation tests drive. Production runs the
/// three phases split apart so one shared board serves many deities:
/// [`resolve_events`] once, [`settle_bets`] per player, then [`refresh_market`]
/// (see `sim::tick_shared`).
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub fn tick_speculations(
    events: &mut Vec<SpeculationEvent>,
    seq: &mut u64,
    player: &mut PlayerState,
    heroes: &[Hero],
    regions: &[Region],
    settlements: &[Settlement],
    chronicle: &mut Chronicle,
    rng: &mut SeededRng,
    data: &GameData,
    year: u32,
    era_number: u32,
    era_progress: f32,
) {
    resolve_events(events, heroes, regions, settlements, year, era_number);
    settle_bets(
        &mut player.bets,
        &mut player.favor,
        events,
        chronicle,
        data,
        year,
    );
    refresh_market(
        events,
        seq,
        heroes,
        regions,
        settlements,
        rng,
        data,
        year,
        era_number,
        era_progress,
    );
}

/// Mark every due event won/lost — world state only, no player. Run once per
/// tick before any deity settles against the board (GDD 5.5).
pub(crate) fn resolve_events(
    events: &mut [SpeculationEvent],
    heroes: &[Hero],
    regions: &[Region],
    settlements: &[Settlement],
    year: u32,
    era_number: u32,
) {
    for event in events.iter_mut() {
        if !event.is_active() {
            continue;
        }
        let outcome = if event.is_satisfied(heroes, regions, settlements, era_number) {
            Some(true)
        } else if year >= event.deadline_year {
            Some(false)
        } else {
            None
        };
        if let Some(won) = outcome {
            event.resolved = Some(won);
        }
    }
}

/// Settle one deity's outstanding wagers against the events that have now
/// resolved: credit each winning bet's locked payout and chronicle the outcome
/// (GDD 5.5). Idempotent — a settled bet is skipped — so it runs safely every
/// tick, once per connected player, against the shared board.
pub(crate) fn settle_bets(
    bets: &mut Vec<Bet>,
    favor: &mut i64,
    events: &[SpeculationEvent],
    chronicle: &mut Chronicle,
    data: &GameData,
    year: u32,
) {
    let text = &data.strings.chronicle;
    for event in events.iter() {
        let Some(won) = event.resolved else { continue };
        for bet in bets
            .iter_mut()
            .filter(|b| b.event_id == event.id && b.resolved.is_none())
        {
            bet.resolved = Some(won);
            let template = if won {
                *favor += bet.potential_payout;
                &text.bet_won
            } else {
                &text.bet_lost
            };
            chronicle.push(
                year,
                EventKind::Divine,
                fill(
                    template,
                    &[
                        ("target", event.target_name.clone()),
                        ("payout", bet.potential_payout.to_string()),
                        ("stake", bet.stake.to_string()),
                    ],
                ),
            );
        }
    }
    prune_bets(bets, data.balance.betting.bet_history_cap);
}

/// Let the crowd keep betting, top the board back up to target, and drop the
/// oldest resolved events — world state only. Run once per tick, after every
/// deity has settled (so a just-resolved event isn't pruned before its backers
/// are paid).
#[allow(clippy::too_many_arguments)]
pub(crate) fn refresh_market(
    events: &mut Vec<SpeculationEvent>,
    seq: &mut u64,
    heroes: &[Hero],
    regions: &[Region],
    settlements: &[Settlement],
    rng: &mut SeededRng,
    data: &GameData,
    year: u32,
    era_number: u32,
    era_progress: f32,
) {
    drift_crowds(
        events,
        heroes,
        regions,
        settlements,
        era_progress,
        data.balance.betting.crowd_drift,
    );
    replenish(
        events,
        seq,
        heroes,
        regions,
        settlements,
        rng,
        data,
        year,
        era_number,
        era_progress,
    );
    prune(events, data.balance.betting.event_cap);
}

#[allow(clippy::too_many_arguments)]
fn replenish(
    events: &mut Vec<SpeculationEvent>,
    seq: &mut u64,
    heroes: &[Hero],
    regions: &[Region],
    settlements: &[Settlement],
    rng: &mut SeededRng,
    data: &GameData,
    year: u32,
    era_number: u32,
    era_progress: f32,
) {
    let target = data.balance.betting.active_events;
    let mut active = events.iter().filter(|e| e.is_active()).count();
    let mut attempts = 0;
    while active < target && attempts < target * 6 {
        attempts += 1;
        if let Some(event) = generate_event(
            seq,
            heroes,
            regions,
            settlements,
            rng,
            data,
            year,
            era_number,
            era_progress,
        ) {
            events.push(event);
            active += 1;
        }
    }
}

/// Pick a living hero to speculate about, weighted toward the renowned and
/// mighty (GDD 5.5): the Observatory watches the heroes who matter most, though
/// any living hero can still be named. Deterministic given the RNG state.
fn pick_notable_hero<'a>(
    heroes: &'a [Hero],
    balance: &BettingBalance,
    rng: &mut SeededRng,
) -> Option<&'a Hero> {
    let alive: Vec<&Hero> = heroes.iter().filter(|h| h.is_alive).collect();
    if alive.is_empty() {
        return None;
    }
    let weight = |h: &Hero| {
        1.0 + h.renown * balance.hero_renown_bias + h.level as f32 * balance.hero_level_bias
    };
    let total: f32 = alive.iter().map(|h| weight(h)).sum();
    let mut roll = rng.next_f32() * total;
    for hero in &alive {
        roll -= weight(hero);
        if roll <= 0.0 {
            return Some(hero);
        }
    }
    alive.last().copied()
}

#[allow(clippy::too_many_arguments)]
fn generate_event(
    seq: &mut u64,
    heroes: &[Hero],
    regions: &[Region],
    settlements: &[Settlement],
    rng: &mut SeededRng,
    data: &GameData,
    year: u32,
    era_number: u32,
    era_progress: f32,
) -> Option<SpeculationEvent> {
    let bet_type = rng.choose(&data.bet_types)?.clone();
    let timeframe = rng.choose(&data.timeframes)?.clone();

    // `origin_region_id` is recorded only for a hero target, so a defection wager
    // knows the home the hero must leave; it stays empty for every other target.
    let (target_id, target_name, origin_region_id) = match bet_type.predicate.target_kind() {
        TargetKind::Hero => {
            let hero = pick_notable_hero(heroes, &data.balance.betting, rng)?;
            (hero.id.clone(), hero.name.clone(), hero.region_id.clone())
        }
        TargetKind::Region => {
            let region = rng.choose(regions)?;
            (region.id.clone(), region.name.clone(), String::new())
        }
        TargetKind::Settlement => {
            let settlement = rng.choose(settlements)?;
            (settlement.id.clone(), settlement.name.clone(), String::new())
        }
        // A world-scale proposition has no entity; its label depends on what it
        // watches — the age, or the shape of the map.
        TargetKind::World => {
            let label = match bet_type.predicate {
                crate::data::BetPredicate::NewRegion => {
                    data.strings.betting.frontier_target.clone()
                }
                _ => data.strings.betting.age_target.clone(),
            };
            (String::new(), label, String::new())
        }
    };

    *seq += 1;
    let b = &data.balance.betting;
    let mut event = SpeculationEvent {
        id: format!("spec-{seq}"),
        bet_type_name: bet_type.name.clone(),
        description: fill(
            &bet_type.description,
            &[
                ("target", target_name.clone()),
                ("threshold", format!("{:.0}", bet_type.threshold)),
            ],
        ),
        predicate: bet_type.predicate,
        threshold: bet_type.threshold,
        target_kind: bet_type.predicate.target_kind(),
        target_id,
        target_name,
        base_odds: bet_type.base_odds,
        timeframe_name: timeframe.name.clone(),
        timeframe_modifier: timeframe.modifier,
        created_year: year,
        deadline_year: year + timeframe.years,
        created_era: era_number,
        created_region_count: regions.len() as u32,
        origin_region_id,
        crowd_yes: 0.0,
        crowd_no: 0.0,
        resolved: None,
    };
    // The crowd of watching deities is wise but imperfect: it stakes toward the
    // proposition's real likelihood, so backing the favourite pays less.
    let likelihood = event.likelihood(heroes, regions, settlements, era_progress);
    (event.crowd_yes, event.crowd_no) = seed_crowd(likelihood, rng, b);
    Some(event)
}

/// Split a random total crowd stake between the outcomes by the crowd's read of
/// the likelihood, wandering from it by up to `crowd_noise`. Deterministic given
/// the RNG state.
fn seed_crowd(likelihood: f32, rng: &mut SeededRng, balance: &BettingBalance) -> (f32, f32) {
    let total = rng.range_f32(balance.crowd_seed_min * 2.0, balance.crowd_seed_max * 2.0);
    let lean =
        (likelihood + rng.range_f32(-balance.crowd_noise, balance.crowd_noise)).clamp(0.05, 0.95);
    (total * lean, total * (1.0 - lean))
}

/// Let the watching deities keep betting: each active event gains `drift` stake
/// split by its *current* likelihood, so the crowd's lean tracks the shifting
/// world rather than staying frozen at its opening seed (GDD 5.5). Deterministic.
fn drift_crowds(
    events: &mut [SpeculationEvent],
    heroes: &[Hero],
    regions: &[Region],
    settlements: &[Settlement],
    era_progress: f32,
    drift: f32,
) {
    for event in events.iter_mut().filter(|e| e.is_active()) {
        let likelihood = event.likelihood(heroes, regions, settlements, era_progress);
        event.crowd_yes += drift * likelihood;
        event.crowd_no += drift * (1.0 - likelihood);
    }
}

/// Drop the oldest resolved events once the store exceeds its cap.
fn prune(events: &mut Vec<SpeculationEvent>, cap: usize) {
    let mut i = 0;
    while events.len() > cap && i < events.len() {
        if events[i].is_active() {
            i += 1;
        } else {
            events.remove(i);
        }
    }
}

/// Keep every pending wager but drop the oldest resolved ones past the cap, so
/// the player's bet history (and the save file) can't grow without bound.
fn prune_bets(bets: &mut Vec<Bet>, cap: usize) {
    let resolved = bets.iter().filter(|b| b.resolved.is_some()).count();
    if resolved <= cap {
        return;
    }
    let mut to_drop = resolved - cap;
    bets.retain(|bet| {
        if to_drop > 0 && bet.resolved.is_some() {
            to_drop -= 1;
            false
        } else {
            true
        }
    });
}

#[cfg(test)]
mod tests {
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
}
