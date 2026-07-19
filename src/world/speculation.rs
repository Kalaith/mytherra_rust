//! Speculation events: the shared-world propositions players bet on (GDD 5.5).
//! Each event denormalizes its predicate + threshold so it can be evaluated
//! without a data lookup, and carries simulated crowd stakes so the crowd-lean
//! payout adjustment is meaningful in this local build.

use crate::data::{BetPredicate, TargetKind};
use crate::world::{Hero, Region, Settlement};
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
    pub fn is_satisfied(
        &self,
        heroes: &[Hero],
        regions: &[Region],
        settlements: &[Settlement],
    ) -> bool {
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
            BetPredicate::RegionDangerAtLeast => self
                .region(regions)
                .map(|r| r.danger >= self.threshold)
                .unwrap_or(false),
            BetPredicate::RegionMagicAtLeast => self
                .region(regions)
                .map(|r| r.magic_affinity >= self.threshold)
                .unwrap_or(false),
            BetPredicate::RegionCrisis => self
                .region(regions)
                .map(|r| r.status.is_crisis())
                .unwrap_or(false),
            // Conquest is the only thing that removes a region, so a target that
            // has vanished from the map was conquered and absorbed.
            BetPredicate::RegionConquered => self.region(regions).is_none(),
            BetPredicate::SettlementPopulationAtLeast => self
                .settlement(settlements)
                .map(|s| s.population >= self.threshold)
                .unwrap_or(false),
            BetPredicate::SettlementProsperityAtLeast => self
                .settlement(settlements)
                .map(|s| s.prosperity >= self.threshold)
                .unwrap_or(false),
        }
    }

    /// Rough current likelihood in [0, 1], used to derive the target odds
    /// modifier so odds react to real world state.
    pub fn likelihood(
        &self,
        heroes: &[Hero],
        regions: &[Region],
        settlements: &[Settlement],
    ) -> f32 {
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
            BetPredicate::RegionDangerAtLeast => self
                .region(regions)
                .map(|r| clamp01(r.danger / self.threshold.max(1.0)))
                .unwrap_or(0.5),
            BetPredicate::RegionMagicAtLeast => self
                .region(regions)
                .map(|r| clamp01(r.magic_affinity / self.threshold.max(1.0)))
                .unwrap_or(0.5),
            BetPredicate::RegionCrisis => self
                .region(regions)
                .map(|r| {
                    clamp01(
                        (r.danger + r.chaos) / 200.0 + if r.status.is_crisis() { 0.5 } else { 0.0 },
                    )
                })
                .unwrap_or(0.5),
            // A weak, crisis-stricken region is the ripe target for conquest; an
            // already-absent one has certainly fallen.
            BetPredicate::RegionConquered => match self.region(regions) {
                None => 1.0,
                Some(r) => clamp01(
                    if r.status.is_crisis() { 0.4 } else { 0.05 }
                        + (100.0 - r.prosperity) / 100.0 * 0.3
                        + r.danger / 100.0 * 0.2,
                ),
            },
            BetPredicate::SettlementPopulationAtLeast => self
                .settlement(settlements)
                .map(|s| clamp01(s.population / self.threshold.max(1.0)))
                .unwrap_or(0.5),
            BetPredicate::SettlementProsperityAtLeast => self
                .settlement(settlements)
                .map(|s| clamp01(s.prosperity / self.threshold.max(1.0)))
                .unwrap_or(0.5),
        }
    }

    fn hero<'a>(&self, heroes: &'a [Hero]) -> Option<&'a Hero> {
        heroes.iter().find(|h| h.id == self.target_id)
    }

    fn region<'a>(&self, regions: &'a [Region]) -> Option<&'a Region> {
        regions.iter().find(|r| r.id == self.target_id)
    }

    fn settlement<'a>(&self, settlements: &'a [Settlement]) -> Option<&'a Settlement> {
        settlements.iter().find(|s| s.id == self.target_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{ClimateType, Culture, GameData, RegionSeed};

    fn region(id: &str) -> Region {
        let balance = GameData::load().unwrap().balance.region;
        Region::from_seed(
            &RegionSeed {
                id: id.to_owned(),
                name: id.to_owned(),
                climate: ClimateType::Temperate,
                culture: Culture::Martial,
                prosperity: 20.0,
                chaos: 80.0,
                danger: 80.0,
                magic_affinity: 40.0,
                population: 3000.0,
                cultural_influence: 40.0,
                divine_resonance: 50.0,
            },
            &balance,
        )
    }

    fn usurpation_event(target_id: &str) -> SpeculationEvent {
        SpeculationEvent {
            id: "spec-1".to_owned(),
            bet_type_name: "Usurpation".to_owned(),
            description: String::new(),
            predicate: BetPredicate::RegionConquered,
            threshold: 0.0,
            target_kind: TargetKind::Region,
            target_id: target_id.to_owned(),
            target_name: target_id.to_owned(),
            base_odds: 4.0,
            timeframe_name: "an age".to_owned(),
            timeframe_modifier: 1.0,
            created_year: 1,
            deadline_year: 50,
            crowd_yes: 1.0,
            crowd_no: 1.0,
            resolved: None,
        }
    }

    #[test]
    fn usurpation_resolves_only_once_the_region_vanishes() {
        let event = usurpation_event("kharzul");
        // While the region stands, the wager is unfulfilled.
        let standing = vec![region("kharzul")];
        assert!(!event.is_satisfied(&[], &standing, &[]));
        // Once conquest removes it from the map, the proposition is satisfied.
        assert!(event.is_satisfied(&[], &[], &[]));
    }

    #[test]
    fn usurpation_likelihood_reflects_vulnerability() {
        let event = usurpation_event("kharzul");
        // A weak, crisis-stricken region reads as more likely to fall than a
        // vanished one reads as certain.
        let weak = vec![region("kharzul")];
        let vulnerable = event.likelihood(&[], &weak, &[]);
        assert!(vulnerable > 0.0 && vulnerable < 1.0);
        assert_eq!(event.likelihood(&[], &[], &[]), 1.0);
    }
}
