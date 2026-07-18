//! Betting content types: bet definitions, confidence tiers, and timeframes
//! (GDD 5.5). All authored in JSON.

use serde::{Deserialize, Serialize};

/// What a speculation event predicts, and against which kind of target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BetPredicate {
    /// The target hero is no longer alive.
    HeroDies,
    /// The target hero reaches at least `threshold` level.
    HeroLevelAtLeast,
    /// The target region's prosperity reaches at least `threshold`.
    RegionProsperityAtLeast,
    /// The target region's chaos reaches at least `threshold`.
    RegionChaosAtLeast,
    /// The target region falls into a crisis status.
    RegionCrisis,
    /// The target settlement's population reaches at least `threshold`.
    SettlementPopulationAtLeast,
    /// The target settlement's prosperity reaches at least `threshold`.
    SettlementProsperityAtLeast,
}

/// Which kind of world entity a predicate targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetKind {
    Hero,
    Region,
    Settlement,
}

impl BetPredicate {
    pub fn target_kind(self) -> TargetKind {
        match self {
            BetPredicate::HeroDies | BetPredicate::HeroLevelAtLeast => TargetKind::Hero,
            BetPredicate::RegionProsperityAtLeast
            | BetPredicate::RegionChaosAtLeast
            | BetPredicate::RegionCrisis => TargetKind::Region,
            BetPredicate::SettlementPopulationAtLeast
            | BetPredicate::SettlementProsperityAtLeast => TargetKind::Settlement,
        }
    }
}

/// An authored bet type: the proposition template and its base odds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BetType {
    pub id: String,
    pub name: String,
    pub base_odds: f32,
    pub predicate: BetPredicate,
    pub threshold: f32,
    pub description: String,
}

/// A confidence tier the player picks when placing a bet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceLevel {
    pub id: String,
    pub name: String,
    pub odds_modifier: f32,
    pub stake_multiplier: f32,
    pub house_edge: f32,
}

/// A wager horizon: how many years until an event expires, and its odds nudge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeframeModifier {
    pub id: String,
    pub name: String,
    pub years: u32,
    pub modifier: f32,
}
