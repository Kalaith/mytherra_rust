//! Speculation events: the shared-world propositions players bet on (GDD 5.5).
//! Each event denormalizes its predicate + threshold so it can be evaluated
//! without a data lookup, and carries simulated crowd stakes so the crowd-lean
//! payout adjustment is meaningful in this local build.

use crate::data::{BetPredicate, TargetKind};
use crate::world::{Hero, Region};
use macroquad_toolkit::math::clamp01;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeculationEvent {
    pub id: String,
    pub bet_type_name: String,
    pub description: String,
    pub predicate: BetPredicate,
    pub threshold: f32,
    pub target_kind: TargetKind,
    pub target_id: String,
    pub target_name: String,
    pub base_odds: f32,
    pub timeframe_name: String,
    pub timeframe_modifier: f32,
    pub created_year: u32,
    pub deadline_year: u32,
    /// Simulated stakes other deities have placed for/against the proposition.
    pub crowd_yes: f32,
    pub crowd_no: f32,
    /// None while active; Some(true) proposition occurred, Some(false) expired.
    pub resolved: Option<bool>,
}

impl SpeculationEvent {
    pub fn is_active(&self) -> bool {
        self.resolved.is_none()
    }

    pub fn crowd_total(&self) -> f32 {
        self.crowd_yes + self.crowd_no
    }

    /// Whether the proposition is currently satisfied by world state.
    pub fn is_satisfied(&self, heroes: &[Hero], regions: &[Region]) -> bool {
        match self.predicate {
            BetPredicate::HeroDies => self.hero(heroes).map(|h| !h.is_alive).unwrap_or(false),
            BetPredicate::HeroLevelAtLeast => self
                .hero(heroes)
                .map(|h| h.is_alive && h.level as f32 >= self.threshold)
                .unwrap_or(false),
            BetPredicate::RegionProsperityAtLeast => self
                .region(regions)
                .map(|r| r.prosperity >= self.threshold)
                .unwrap_or(false),
            BetPredicate::RegionChaosAtLeast => self
                .region(regions)
                .map(|r| r.chaos >= self.threshold)
                .unwrap_or(false),
            BetPredicate::RegionCrisis => self
                .region(regions)
                .map(|r| r.status.is_crisis())
                .unwrap_or(false),
        }
    }

    /// Rough current likelihood in [0, 1], used to derive the target odds
    /// modifier so odds react to real world state.
    pub fn likelihood(&self, heroes: &[Hero], regions: &[Region]) -> f32 {
        match self.predicate {
            BetPredicate::HeroDies => self
                .hero(heroes)
                .map(|h| {
                    if !h.is_alive {
                        1.0
                    } else {
                        clamp01(h.age as f32 / 90.0)
                    }
                })
                .unwrap_or(0.5),
            BetPredicate::HeroLevelAtLeast => self
                .hero(heroes)
                .map(|h| clamp01(h.level as f32 / self.threshold.max(1.0)))
                .unwrap_or(0.5),
            BetPredicate::RegionProsperityAtLeast => self
                .region(regions)
                .map(|r| clamp01(r.prosperity / self.threshold.max(1.0)))
                .unwrap_or(0.5),
            BetPredicate::RegionChaosAtLeast => self
                .region(regions)
                .map(|r| clamp01(r.chaos / self.threshold.max(1.0)))
                .unwrap_or(0.5),
            BetPredicate::RegionCrisis => self
                .region(regions)
                .map(|r| {
                    clamp01(
                        (r.danger + r.chaos) / 200.0 + if r.status.is_crisis() { 0.5 } else { 0.0 },
                    )
                })
                .unwrap_or(0.5),
        }
    }

    fn hero<'a>(&self, heroes: &'a [Hero]) -> Option<&'a Hero> {
        heroes.iter().find(|h| h.id == self.target_id)
    }

    fn region<'a>(&self, regions: &'a [Region]) -> Option<&'a Region> {
        regions.iter().find(|r| r.id == self.target_id)
    }
}
