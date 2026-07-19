//! Betting content types: bet definitions, confidence tiers, and timeframes
//! (GDD 5.5). All authored in JSON.

use serde::{Deserialize, Serialize};

/// What a speculation event predicts, and against which kind of target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BetPredicate {
    /// The target hero is no longer alive.
    #[default]
    HeroDies,
    /// The target hero reaches at least `threshold` level.
    HeroLevelAtLeast,
    /// The target hero (still living) accrues at least `threshold` renown —
    /// a legend in the making (GDD 5.4).
    HeroRenownAtLeast,
    /// The target region's prosperity reaches at least `threshold`.
    RegionProsperityAtLeast,
    /// The target region's chaos reaches at least `threshold`.
    RegionChaosAtLeast,
    /// The target region's danger reaches at least `threshold`.
    RegionDangerAtLeast,
    /// The target region's magic reaches at least `threshold`.
    RegionMagicAtLeast,
    /// The target region's cultural influence reaches at least `threshold` —
    /// a rising cultural centre (fed by trade and myth, GDD 5.2).
    RegionCultureAtLeast,
    /// The target region falls into a crisis status.
    RegionCrisis,
    /// The target region is conquered and absorbed by another (GDD 5.2) —
    /// satisfied when it no longer exists on the map.
    RegionConquered,
    /// The target settlement's population reaches at least `threshold`.
    SettlementPopulationAtLeast,
    /// The target settlement's prosperity reaches at least `threshold`.
    SettlementProsperityAtLeast,
    /// The present age ends (a new era begins) before the wager expires — a
    /// world-scale proposition with no entity target (GDD 5.7).
    AgeEnds,
}

/// Which kind of world entity a predicate targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetKind {
    Hero,
    Region,
    Settlement,
    /// The world as a whole — no single entity (e.g. the era ending).
    World,
}

impl BetPredicate {
    pub fn target_kind(self) -> TargetKind {
        match self {
            BetPredicate::HeroDies
            | BetPredicate::HeroLevelAtLeast
            | BetPredicate::HeroRenownAtLeast => TargetKind::Hero,
            BetPredicate::RegionProsperityAtLeast
            | BetPredicate::RegionChaosAtLeast
            | BetPredicate::RegionDangerAtLeast
            | BetPredicate::RegionMagicAtLeast
            | BetPredicate::RegionCultureAtLeast
            | BetPredicate::RegionCrisis
            | BetPredicate::RegionConquered => TargetKind::Region,
            BetPredicate::SettlementPopulationAtLeast
            | BetPredicate::SettlementProsperityAtLeast => TargetKind::Settlement,
            BetPredicate::AgeEnds => TargetKind::World,
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
