//! Speculation-event lifecycle (GDD 5.5): resolve due events and their bets,
//! then generate fresh events to keep the Observatory stocked. Event generation
//! uses the world RNG; bet payouts were locked at placement, so resolution here
//! is a pure win/lose credit.

use crate::data::{fill, GameData, TargetKind};
use crate::world::{Chronicle, EventKind, Hero, PlayerState, Region, Settlement, SpeculationEvent};
use macroquad_toolkit::rng::SeededRng;

/// Resolve, replenish, and prune speculation events for one tick.
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
) {
    resolve_due(
        events,
        player,
        heroes,
        regions,
        settlements,
        chronicle,
        data,
        year,
    );
    replenish(events, seq, heroes, regions, settlements, rng, data, year);
    prune(events, data.balance.betting.event_cap);
}

#[allow(clippy::too_many_arguments)]
fn resolve_due(
    events: &mut [SpeculationEvent],
    player: &mut PlayerState,
    heroes: &[Hero],
    regions: &[Region],
    settlements: &[Settlement],
    chronicle: &mut Chronicle,
    data: &GameData,
    year: u32,
) {
    let text = &data.strings.chronicle;
    for event in events.iter_mut() {
        if !event.is_active() {
            continue;
        }
        let outcome = if event.is_satisfied(heroes, regions, settlements) {
            Some(true)
        } else if year >= event.deadline_year {
            Some(false)
        } else {
            None
        };
        let Some(won) = outcome else { continue };
        event.resolved = Some(won);

        for bet in player
            .bets
            .iter_mut()
            .filter(|b| b.event_id == event.id && b.resolved.is_none())
        {
            bet.resolved = Some(won);
            let template = if won {
                // Credit the locked payout directly (disjoint field from bets).
                player.favor += bet.potential_payout;
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
) {
    let target = data.balance.betting.active_events;
    let mut active = events.iter().filter(|e| e.is_active()).count();
    let mut attempts = 0;
    while active < target && attempts < target * 6 {
        attempts += 1;
        if let Some(event) = generate_event(seq, heroes, regions, settlements, rng, data, year) {
            events.push(event);
            active += 1;
        }
    }
}

fn generate_event(
    seq: &mut u64,
    heroes: &[Hero],
    regions: &[Region],
    settlements: &[Settlement],
    rng: &mut SeededRng,
    data: &GameData,
    year: u32,
) -> Option<SpeculationEvent> {
    let bet_type = rng.choose(&data.bet_types)?.clone();
    let timeframe = rng.choose(&data.timeframes)?.clone();

    let (target_id, target_name) = match bet_type.predicate.target_kind() {
        TargetKind::Hero => {
            let alive: Vec<&Hero> = heroes.iter().filter(|h| h.is_alive).collect();
            let hero = *rng.choose(&alive)?;
            (hero.id.clone(), hero.name.clone())
        }
        TargetKind::Region => {
            let region = rng.choose(regions)?;
            (region.id.clone(), region.name.clone())
        }
        TargetKind::Settlement => {
            let settlement = rng.choose(settlements)?;
            (settlement.id.clone(), settlement.name.clone())
        }
    };

    *seq += 1;
    let b = &data.balance.betting;
    Some(SpeculationEvent {
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
        crowd_yes: rng.range_f32(b.crowd_seed_min, b.crowd_seed_max),
        crowd_no: rng.range_f32(b.crowd_seed_min, b.crowd_seed_max),
        resolved: None,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldState;

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
        );
        let active = world.speculations.iter().filter(|e| e.is_active()).count();
        assert_eq!(active, data.balance.betting.active_events);
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
            );
        }
        let mut ids: Vec<&str> = world.speculations.iter().map(|e| e.id.as_str()).collect();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count, "ids must be unique");
    }
}
