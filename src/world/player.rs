//! The local player's private economy: Divine Favor, level, and experience.
//!
//! Per GDD Pillar 3, favor is per-player and private even though the world is
//! shared. In this local build there is a single player, but the type keeps
//! that boundary explicit so a future server can own one row per account.

use crate::data::{ChampionBalance, ChampionFocus, GameConfig, PlayerBalance};
use crate::world::{Bet, Champion};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub favor: i64,
    pub level: u32,
    pub experience: i64,
    /// Running total of favor spent, for the dashboard chronicle.
    pub favor_spent: i64,
    /// Number of divine nudges the player has performed.
    pub nudges: u32,
    /// The player's cultivated champion roster (GDD 5.4).
    pub champions: Vec<Champion>,
    /// The player's placed bets (GDD 5.5).
    pub bets: Vec<Bet>,
    /// Unlocked-achievement state; reconciled with the current definitions on
    /// load. `serde(default)` keeps pre-achievement saves loadable.
    #[serde(default)]
    pub achievements: macroquad_toolkit::achievements::Achievements,
}

impl PlayerState {
    pub fn new(config: &GameConfig) -> Self {
        Self {
            favor: config.starting_favor,
            level: 1,
            experience: 0,
            favor_spent: 0,
            nudges: 0,
            champions: Vec::new(),
            bets: Vec::new(),
            achievements: macroquad_toolkit::achievements::Achievements::new(),
        }
    }

    /// Debit favor for a wager stake (no experience / nudge accounting, unlike
    /// a divine action). Returns false without mutating if unaffordable.
    pub fn place_stake(&mut self, stake: i64) -> bool {
        if !self.can_afford(stake) {
            return false;
        }
        self.favor -= stake;
        self.favor_spent += stake;
        true
    }

    pub fn is_champion(&self, hero_id: &str) -> bool {
        self.champions.iter().any(|c| c.hero_id == hero_id)
    }

    pub fn champion_mut(&mut self, hero_id: &str) -> Option<&mut Champion> {
        self.champions.iter_mut().find(|c| c.hero_id == hero_id)
    }

    /// Designate a hero as a champion if there is room and they aren't already
    /// one. Returns false without mutating otherwise.
    pub fn designate_champion(
        &mut self,
        hero_id: &str,
        focus: ChampionFocus,
        balance: &ChampionBalance,
    ) -> bool {
        if self.is_champion(hero_id) || self.champions.len() >= balance.max_roster {
            return false;
        }
        self.champions
            .push(Champion::designate(hero_id.to_owned(), focus));
        true
    }

    pub fn can_afford(&self, cost: i64) -> bool {
        self.favor >= cost
    }

    /// Spend favor on a divine act. Returns false without mutating if the player
    /// cannot afford it.
    pub fn spend(&mut self, cost: i64, balance: &PlayerBalance) -> bool {
        if !self.can_afford(cost) {
            return false;
        }
        self.favor -= cost;
        self.favor_spent += cost;
        self.nudges += 1;
        self.gain_experience(cost, balance);
        true
    }

    /// The deity's favor ceiling at its current standing: the base plus a bonus
    /// per level attained (GDD 5.1).
    pub fn max_favor(&self, config: &GameConfig, balance: &PlayerBalance) -> i64 {
        config.max_favor + (self.level as i64 - 1) * balance.max_favor_per_level
    }

    /// Passive per-tick favor recovery at the current standing.
    pub fn favor_recovery(&self, config: &GameConfig, balance: &PlayerBalance) -> i64 {
        config.favor_per_tick + (self.level as i64 - 1) * balance.favor_per_tick_per_level
    }

    /// Passive per-tick favor recovery, capped at the standing's ceiling.
    pub fn recover(&mut self, config: &GameConfig, balance: &PlayerBalance) {
        self.favor = (self.favor + self.favor_recovery(config, balance))
            .min(self.max_favor(config, balance));
    }

    fn gain_experience(&mut self, amount: i64, balance: &PlayerBalance) {
        self.experience += amount;
        while self.experience >= self.next_level_cost(balance) {
            self.experience -= self.next_level_cost(balance);
            self.level += 1;
        }
    }

    /// Experience required to advance from the current level (tuned in
    /// `balance.json`).
    pub fn next_level_cost(&self, balance: &PlayerBalance) -> i64 {
        balance.level_base_cost + (self.level as i64 - 1) * balance.level_cost_step
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> GameConfig {
        GameConfig {
            game_name: "mytherra".to_owned(),
            display_name: "Mytherra".to_owned(),
            save_slot: "world".to_owned(),
            version: "0.1.0".to_owned(),
            starting_favor: 140,
            favor_per_tick: 10,
            max_favor: 600,
            seconds_per_tick: 10.0,
            autosave_every_ticks: 6,
            start_year: 1,
            world_seed: 1,
        }
    }

    fn player_balance() -> PlayerBalance {
        PlayerBalance {
            level_base_cost: 100,
            level_cost_step: 60,
            max_favor_per_level: 40,
            favor_per_tick_per_level: 1,
        }
    }

    #[test]
    fn spending_debits_and_tracks() {
        let mut player = PlayerState::new(&config());
        assert!(player.spend(15, &player_balance()));
        assert_eq!(player.favor, 125);
        assert_eq!(player.favor_spent, 15);
        assert_eq!(player.nudges, 1);
    }

    #[test]
    fn cannot_overspend() {
        let mut player = PlayerState::new(&config());
        assert!(!player.spend(10_000, &player_balance()));
        assert_eq!(player.favor, 140);
    }

    #[test]
    fn recovery_respects_ceiling() {
        let cfg = config();
        let bal = player_balance();
        let mut player = PlayerState::new(&cfg);
        player.favor = cfg.max_favor - 5;
        player.recover(&cfg, &bal);
        assert_eq!(player.favor, cfg.max_favor);
    }

    #[test]
    fn a_higher_standing_holds_and_recovers_more_favor() {
        let cfg = config();
        let bal = player_balance();
        let mut player = PlayerState::new(&cfg);
        let base_cap = player.max_favor(&cfg, &bal);
        let base_recovery = player.favor_recovery(&cfg, &bal);

        player.level = 4; // three levels past the first
        assert_eq!(
            player.max_favor(&cfg, &bal),
            base_cap + 3 * bal.max_favor_per_level
        );
        assert_eq!(
            player.favor_recovery(&cfg, &bal),
            base_recovery + 3 * bal.favor_per_tick_per_level
        );

        // Recovery now fills toward the raised ceiling, not the base one.
        player.favor = base_cap;
        player.recover(&cfg, &bal);
        assert!(player.favor > base_cap);
    }

    #[test]
    fn spending_grants_levels() {
        let mut player = PlayerState::new(&config());
        player.favor = 10_000;
        for _ in 0..20 {
            player.spend(30, &player_balance());
        }
        assert!(player.level > 1);
    }
}
