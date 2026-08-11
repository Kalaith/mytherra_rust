use super::*;
use crate::data::GameData;

fn balance() -> BettingBalance {
    GameData::load().unwrap().balance.betting
}

#[test]
fn odds_respect_floor() {
    let b = balance();
    // Tiny base with a near-certain modifier should still floor at min_odds.
    let odds = house_odds(1.0, 0.1, 0.85, 0.6, b.min_odds);
    assert!(odds >= b.min_odds);
}

#[test]
fn heavy_crowd_pays_less_than_thin_crowd() {
    let b = balance();
    let heavy = crowd_lean_factor(90.0, 100.0, &b);
    let thin = crowd_lean_factor(10.0, 100.0, &b);
    assert!(heavy < thin);
}

#[test]
fn payout_never_below_stake_plus_one() {
    let b = balance();
    let conf = GameData::load().unwrap().confidence_levels[0].clone();
    assert!(payout(50, 1.1, &conf, &b) >= 51);
}

fn bet(stake: i64, payout: i64, resolved: Option<bool>) -> Bet {
    Bet {
        event_id: "e".to_owned(),
        predicate: crate::data::BetPredicate::default(),
        bet_type_name: String::new(),
        target_name: String::new(),
        confidence_name: String::new(),
        stake,
        potential_payout: payout,
        odds: 2.0,
        placed_year: 1,
        deadline_year: 5,
        resolved,
    }
}

#[test]
fn record_tallies_wins_losses_and_net_favor() {
    let bets = vec![
        bet(20, 50, Some(true)), // +30
        bet(30, 90, Some(true)), // +60
        bet(40, 0, Some(false)), // -40
        bet(25, 0, None),        // pending, ignored
    ];
    let r = bet_record(&bets);
    assert_eq!(r.won, 2);
    assert_eq!(r.lost, 1);
    assert_eq!(r.pending, 1);
    assert_eq!(r.net, 30 + 60 - 40);
}

#[test]
fn an_empty_history_is_a_blank_record() {
    assert_eq!(bet_record(&[]), BetRecord::default());
}
