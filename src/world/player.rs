//! The local player's private economy: Divine Favor, level, and experience.
//!
//! Per GDD Pillar 3, favor is per-player and private even though the world is
//! shared. In this local build there is a single player, but the type keeps
//! that boundary explicit so a future server can own one row per account.

use crate::data::GameConfig;
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
}

impl PlayerState {
    pub fn new(config: &GameConfig) -> Self {
        Self {
            favor: config.starting_favor,
            level: 1,
            experience: 0,
            favor_spent: 0,
            nudges: 0,
        }
    }

    pub fn can_afford(&self, cost: i64) -> bool {
        self.favor >= cost
    }

    /// Spend favor on a divine act. Returns false without mutating if the player
    /// cannot afford it.
    pub fn spend(&mut self, cost: i64) -> bool {
        if !self.can_afford(cost) {
            return false;
        }
        self.favor -= cost;
        self.favor_spent += cost;
        self.nudges += 1;
        self.gain_experience(cost);
        true
    }

    /// Passive per-tick favor recovery, capped at the configured ceiling.
    pub fn recover(&mut self, config: &GameConfig) {
        self.favor = (self.favor + config.favor_per_tick).min(config.max_favor);
    }

    fn gain_experience(&mut self, amount: i64) {
        self.experience += amount;
        while self.experience >= self.next_level_cost() {
            self.experience -= self.next_level_cost();
            self.level += 1;
        }
    }

    /// Experience required to advance from the current level.
    pub fn next_level_cost(&self) -> i64 {
        100 + (self.level as i64 - 1) * 60
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
            start_year: 1,
        }
    }

    #[test]
    fn spending_debits_and_tracks() {
        let mut player = PlayerState::new(&config());
        assert!(player.spend(15));
        assert_eq!(player.favor, 125);
        assert_eq!(player.favor_spent, 15);
        assert_eq!(player.nudges, 1);
    }

    #[test]
    fn cannot_overspend() {
        let mut player = PlayerState::new(&config());
        assert!(!player.spend(10_000));
        assert_eq!(player.favor, 140);
    }

    #[test]
    fn recovery_respects_ceiling() {
        let cfg = config();
        let mut player = PlayerState::new(&cfg);
        player.favor = cfg.max_favor - 5;
        player.recover(&cfg);
        assert_eq!(player.favor, cfg.max_favor);
    }

    #[test]
    fn spending_grants_levels() {
        let mut player = PlayerState::new(&config());
        player.favor = 10_000;
        for _ in 0..20 {
            player.spend(30);
        }
        assert!(player.level > 1);
    }
}
