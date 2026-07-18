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
mod tests {
    use super::*;

    fn balance() -> ChampionBalance {
        crate::data::GameData::load().unwrap().balance.champion
    }

    #[test]
    fn rank_never_decreases_and_respects_cap() {
        let b = balance();
        let mut champ = Champion::designate("h".to_owned(), ChampionFocus::Valor);
        champ.bond = 250.0;
        champ.quests = 40;
        champ.recompute_rank(&b);
        assert_eq!(champ.rank, b.rank_cap);
        champ.bond = 0.0;
        champ.quests = 0;
        champ.recompute_rank(&b);
        assert_eq!(champ.rank, b.rank_cap, "rank must not drop");
    }

    #[test]
    fn cultivate_cost_grows_with_rank() {
        let b = balance();
        let mut champ = Champion::designate("h".to_owned(), ChampionFocus::Devotion);
        let low = champ.cultivate_cost(&b);
        champ.rank = 5;
        assert!(champ.cultivate_cost(&b) > low);
    }

    #[test]
    fn quest_step_is_clamped() {
        let b = balance();
        let champ = Champion::designate("h".to_owned(), ChampionFocus::Wisdom);
        let step = champ.quest_step(1, &b);
        assert!(step >= b.quest.min && step <= b.quest.max);
    }
}
