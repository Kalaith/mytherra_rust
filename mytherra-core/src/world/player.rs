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

    /// Passive per-tick favor recovery, capped at the standing's ceiling. The
    /// `tithe` is the extra favor the world's faithful lands pour back this tick
    /// (GDD 5.1 <-> 5.4), computed from region resonance by the sim; folding it in
    /// here keeps the single favor ceiling authoritative.
    pub fn recover(&mut self, tithe: i64, config: &GameConfig, balance: &PlayerBalance) {
        self.favor = (self.favor + self.favor_recovery(config, balance) + tithe)
            .min(self.max_favor(config, balance));
    }

    /// Award experience toward the deity's next standing, leveling up as many
    /// times as the total clears. Used both by spending favor (§5.1) and by
    /// unlocking achievements, whose milestones elevate the god.
    pub fn gain_experience(&mut self, amount: i64, balance: &PlayerBalance) {
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
mod tests;
