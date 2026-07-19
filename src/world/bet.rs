//! Player bets and the pure odds / payout math (GDD 5.5).
//!
//! Odds are locked at placement time (fixed-odds), so payout is deterministic
//! from the stored values regardless of how the world moves afterward.

use crate::data::{BettingBalance, ConfidenceLevel};
use crate::world::SpeculationEvent;
use serde::{Deserialize, Serialize};

/// A priced-up wager preview: what the player would get for a given event,
/// confidence, and stake. Computed identically for the UI preview and the
/// actual placement so they never disagree.
#[derive(Debug, Clone, Copy)]
pub struct BetQuote {
    /// Effective odds after the crowd-lean adjustment.
    pub odds: f32,
    /// Favor credited on a win.
    pub payout: i64,
    /// Share of stake on the "yes" outcome, as a percentage.
    pub crowd_pct: f32,
}

/// Price a wager on an event. `likelihood` is the event's current world-state
/// likelihood (`SpeculationEvent::likelihood`); `stake` is added to the yes
/// side so the crowd-lean reflects the bet being placed.
pub fn quote_event(
    event: &SpeculationEvent,
    likelihood: f32,
    confidence: &ConfidenceLevel,
    stake: i64,
    balance: &BettingBalance,
) -> BetQuote {
    let target_mod = target_modifier(likelihood, balance);
    let odds = house_odds(
        event.base_odds,
        confidence.odds_modifier,
        event.timeframe_modifier,
        target_mod,
        balance.min_odds,
    );
    let crowd_yes = event.crowd_yes + stake as f32;
    let crowd_total = event.crowd_total() + stake as f32;
    let effective = odds * crowd_lean_factor(crowd_yes, crowd_total, balance);
    BetQuote {
        odds: (effective * 100.0).round() / 100.0,
        payout: payout(stake, effective, confidence, balance),
        crowd_pct: if crowd_total > 0.0 {
            crowd_yes / crowd_total * 100.0
        } else {
            50.0
        },
    }
}

/// A wager the player has placed on a speculation event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bet {
    pub event_id: String,
    pub bet_type_name: String,
    pub target_name: String,
    pub confidence_name: String,
    pub stake: i64,
    /// Payout credited on a win (0 stake is never allowed).
    pub potential_payout: i64,
    /// Effective odds shown to the player (house odds after crowd-lean).
    pub odds: f32,
    pub placed_year: u32,
    pub deadline_year: u32,
    /// None while pending; Some(true) won, Some(false) lost.
    pub resolved: Option<bool>,
}

/// The player's betting track record, summarized from their wager history.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BetRecord {
    pub won: u32,
    pub lost: u32,
    pub pending: u32,
    /// Net favor from settled wagers: `+(payout - stake)` per win, `-stake` per
    /// loss. Pending wagers don't count until they resolve.
    pub net: i64,
}

/// Summarize a player's wagers into a win/loss/pending tally and net favor.
pub fn bet_record(bets: &[Bet]) -> BetRecord {
    let mut record = BetRecord::default();
    for bet in bets {
        match bet.resolved {
            Some(true) => {
                record.won += 1;
                record.net += bet.potential_payout - bet.stake;
            }
            Some(false) => {
                record.lost += 1;
                record.net -= bet.stake;
            }
            None => record.pending += 1,
        }
    }
    record
}

/// House odds from the fair-value formula, floored and rounded (GDD 5.5).
pub fn house_odds(
    base_odds: f32,
    confidence_odds_modifier: f32,
    timeframe_modifier: f32,
    target_modifier: f32,
    min_odds: f32,
) -> f32 {
    let odds = base_odds * confidence_odds_modifier * timeframe_modifier * target_modifier;
    (odds.max(min_odds) * 100.0).round() / 100.0
}

/// World-state-derived target modifier: likelier propositions pay less.
pub fn target_modifier(likelihood: f32, balance: &BettingBalance) -> f32 {
    (1.5 - likelihood.clamp(0.0, 1.0)).clamp(balance.target_mod_min, balance.target_mod_max)
}

/// Crowd-lean payout factor: heavily-backed outcomes pay less, thin ones more.
/// `clamp(min, max, 1 / (0.5 + crowd_lean))` (GDD 5.5).
pub fn crowd_lean_factor(crowd_yes: f32, crowd_total: f32, balance: &BettingBalance) -> f32 {
    let lean = if crowd_total > 0.0 {
        crowd_yes / crowd_total
    } else {
        0.5
    };
    (1.0 / (0.5 + lean)).clamp(balance.crowd_lean_min, balance.crowd_lean_max)
}

/// Gross payout on a win: applies the confidence stake multiplier and house
/// edge, clamped, with a minimum of `stake + 1` (GDD 5.5).
pub fn payout(
    stake: i64,
    effective_odds: f32,
    confidence: &ConfidenceLevel,
    balance: &BettingBalance,
) -> i64 {
    let raw_multiplier = effective_odds * confidence.stake_multiplier;
    let gross_multiplier = (1.0 + (raw_multiplier - 1.0) * confidence.house_edge)
        .clamp(balance.payout_min_mult, balance.payout_max_mult);
    ((stake as f32 * gross_multiplier).floor() as i64).max(stake + 1)
}

#[cfg(test)]
mod tests {
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
}
