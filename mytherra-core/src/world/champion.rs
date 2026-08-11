//! A player-cultivated champion: a hero the player has bonded with and is
//! guiding through quests (GDD 5.4). Player-scoped state (GDD 6).

use crate::data::{ChampionBalance, ChampionFocus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Champion {
    /// Id of the hero this champion is (references `world.heroes`).
    pub hero_id: String,
    pub focus: ChampionFocus,
    pub bond: f32,
    pub rank: u32,
    /// Progress toward the current quest (0..goal).
    pub quest_progress: f32,
    pub quests: u32,
}

impl Champion {
    pub fn designate(hero_id: String, focus: ChampionFocus) -> Self {
        Self {
            hero_id,
            focus,
            bond: 0.0,
            rank: 1,
            quest_progress: 0.0,
            quests: 0,
        }
    }

    /// Rank from bond and completed quests, monotonic and capped (GDD 5.4):
    /// `min(cap, max(current, 1 + bond/per_bond, 1 + quests/per_quests))`.
    pub fn recompute_rank(&mut self, balance: &ChampionBalance) {
        let from_bond = 1.0 + self.bond / balance.rank_per_bond;
        let from_quests = 1.0 + self.quests as f32 / balance.rank_per_quests;
        let candidate = from_bond.max(from_quests).floor() as u32;
        self.rank = candidate.max(self.rank).min(balance.rank_cap);
    }

    /// Favor cost to cultivate once: `base + rank*5 + focus_cost_modifier`.
    pub fn cultivate_cost(&self, balance: &ChampionBalance) -> i64 {
        balance.base_cultivate_cost
            + self.rank as i64 * 5
            + balance.focuses.get(self.focus).cost_modifier
    }

    /// Per-tick quest progress, clamped (GDD 5.4). `hero_level` comes from the
    /// referenced hero.
    pub fn quest_step(&self, hero_level: u32, balance: &ChampionBalance) -> f32 {
        let q = &balance.quest;
        let focus_bonus = balance.focuses.get(self.focus).quest_bonus;
        let raw = q.base
            + self.rank as f32 * q.rank_mult
            + self.bond / q.bond_div
            + hero_level as f32 / q.level_div
            + focus_bonus;
        raw.clamp(q.min, q.max)
    }
}

#[cfg(test)]
mod tests;
