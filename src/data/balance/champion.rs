//! Champion cultivation, questing, and rivalry tuning (GDD 5.4).

use crate::data::champion::ChampionFocus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChampionBalance {
    pub max_roster: usize,
    pub designate_cost: i64,
    pub cultivate_bond_gain: f32,
    pub base_cultivate_cost: i64,
    pub rank_per_bond: f32,
    pub rank_per_quests: f32,
    pub rank_cap: u32,
    pub quest: QuestParams,
    pub rivalry: RivalryParams,
    pub focuses: ChampionFocuses,
}

/// Per-tick quest-progress formula parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestParams {
    pub base: f32,
    pub rank_mult: f32,
    pub bond_div: f32,
    pub level_div: f32,
    pub min: f32,
    pub max: f32,
    pub goal: f32,
}

/// Deterministic rivalry-resolution parameters (strength vs. threat).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RivalryParams {
    pub strength_bond: f32,
    pub strength_rank: f32,
    pub strength_level: f32,
    pub threat_danger: f32,
    pub threat_chaos_div: f32,
    pub resolved_danger: f32,
    pub resolved_chaos: f32,
    pub resolved_prosperity: f32,
    /// Secession pressure a resolved rivalry bleeds off (champion holds the
    /// region together) vs. what an escalation feeds it (GDD 5.4 ↔ 5.2).
    pub resolved_strife: f32,
    pub escalated_danger: f32,
    pub escalated_chaos: f32,
    pub escalated_strife: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChampionFocuses {
    pub valor: FocusParams,
    pub wisdom: FocusParams,
    pub devotion: FocusParams,
}

impl ChampionFocuses {
    pub fn get(&self, focus: ChampionFocus) -> &FocusParams {
        match focus {
            ChampionFocus::Valor => &self.valor,
            ChampionFocus::Wisdom => &self.wisdom,
            ChampionFocus::Devotion => &self.devotion,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusParams {
    pub cost_modifier: i64,
    pub quest_bonus: f32,
    /// Signature stat deltas a champion of this focus adds when it *resolves* a
    /// rivalry, so the focus shapes what kind of impact the champion has (GDD
    /// 5.4), not just how fast it quests.
    pub resolve_prosperity: f32,
    pub resolve_danger: f32,
    pub resolve_magic: f32,
}
