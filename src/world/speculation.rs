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
    /// Era number when the wager was opened; an `AgeEnds` proposition is met once
    /// the world's era has advanced past it. `serde(default)` keeps old saves loadable.
    #[serde(default)]
    pub created_era: u32,
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
    /// `era_number` is the world's current era, needed only by world-scale
    /// propositions (`AgeEnds`).
    pub fn is_satisfied(
        &self,
        heroes: &[Hero],
        regions: &[Region],
        settlements: &[Settlement],
        era_number: u32,
    ) -> bool {
        match self.predicate {
            BetPredicate::HeroDies => self.hero(heroes).map(|h| !h.is_alive).unwrap_or(false),
            BetPredicate::HeroLevelAtLeast => self
                .hero(heroes)
                .map(|h| h.is_alive && h.level as f32 >= self.threshold)
                .unwrap_or(false),
            BetPredicate::HeroRenownAtLeast => self
                .hero(heroes)
                .map(|h| h.is_alive && h.renown >= self.threshold)
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
            BetPredicate::RegionCultureAtLeast => self
                .region(regions)
                .map(|r| r.cultural_influence >= self.threshold)
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
            // The age has turned since the wager opened.
            BetPredicate::AgeEnds => era_number > self.created_era,
        }
    }

    /// Rough current likelihood in [0, 1], used to derive the target odds
    /// modifier so odds react to real world state. `era_progress` is the era's
    /// pressure over its breaking threshold, read only by `AgeEnds`.
    pub fn likelihood(
        &self,
        heroes: &[Hero],
        regions: &[Region],
        settlements: &[Settlement],
        era_progress: f32,
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
            // A hero already alive with rising fame trends toward the bar; a dead
            // one can never reach it.
            BetPredicate::HeroRenownAtLeast => self
                .hero(heroes)
                .map(|h| {
                    if h.is_alive {
                        clamp01(h.renown / self.threshold.max(1.0))
                    } else {
                        0.0
                    }
                })
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
            BetPredicate::RegionCultureAtLeast => self
                .region(regions)
                .map(|r| clamp01(r.cultural_influence / self.threshold.max(1.0)))
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
            // The nearer the era is to breaking, the likelier the age ends soon;
            // squared so a calm age reads as genuinely unlikely to turn.
            BetPredicate::AgeEnds => clamp01(era_progress * era_progress),
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
            created_era: 1,
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
        assert!(!event.is_satisfied(&[], &standing, &[], 1));
        // Once conquest removes it from the map, the proposition is satisfied.
        assert!(event.is_satisfied(&[], &[], &[], 1));
    }

    fn hero(id: &str, renown: f32, alive: bool) -> Hero {
        Hero {
            id: id.to_owned(),
            name: id.to_owned(),
            role: crate::data::HeroRole::Warrior,
            region_id: "kharzul".to_owned(),
            level: 10,
            age: 40,
            is_alive: alive,
            renown,
        }
    }

    fn legend_event(target_id: &str, threshold: f32) -> SpeculationEvent {
        let mut e = usurpation_event(target_id);
        e.predicate = BetPredicate::HeroRenownAtLeast;
        e.target_kind = TargetKind::Hero;
        e.threshold = threshold;
        e
    }

    #[test]
    fn legend_resolves_only_for_a_living_hero_past_the_renown_bar() {
        let event = legend_event("brogan", 100.0);
        // Below the bar: unfulfilled.
        assert!(!event.is_satisfied(&[hero("brogan", 60.0, true)], &[], &[], 1));
        // At/over the bar while alive: a legend.
        assert!(event.is_satisfied(&[hero("brogan", 120.0, true)], &[], &[], 1));
        // A fallen hero can never win the wager, however renowned.
        assert!(!event.is_satisfied(&[hero("brogan", 200.0, false)], &[], &[], 1));
    }

    #[test]
    fn legend_likelihood_scales_with_renown_and_zeroes_on_death() {
        let event = legend_event("brogan", 100.0);
        let rising = event.likelihood(&[hero("brogan", 50.0, true)], &[], &[], 0.0);
        assert!((rising - 0.5).abs() < 0.01, "halfway to the bar reads ~0.5");
        assert_eq!(
            event.likelihood(&[hero("brogan", 40.0, false)], &[], &[], 0.0),
            0.0
        );
    }

    #[test]
    fn usurpation_likelihood_reflects_vulnerability() {
        let event = usurpation_event("kharzul");
        // A weak, crisis-stricken region reads as more likely to fall than a
        // vanished one reads as certain.
        let weak = vec![region("kharzul")];
        let vulnerable = event.likelihood(&[], &weak, &[], 0.0);
        assert!(vulnerable > 0.0 && vulnerable < 1.0);
        assert_eq!(event.likelihood(&[], &[], &[], 0.0), 1.0);
    }

    #[test]
    fn a_renaissance_resolves_when_culture_clears_the_bar() {
        // `region()` seeds cultural_influence at 40.
        let mut event = usurpation_event("kharzul");
        event.predicate = BetPredicate::RegionCultureAtLeast;
        let regions = vec![region("kharzul")];

        event.threshold = 30.0;
        assert!(
            event.is_satisfied(&[], &regions, &[], 1),
            "40 clears a bar of 30"
        );
        event.threshold = 60.0;
        assert!(
            !event.is_satisfied(&[], &regions, &[], 1),
            "40 falls short of 60"
        );
        // Likelihood tracks the ratio to the bar.
        assert!((event.likelihood(&[], &regions, &[], 0.0) - 40.0 / 60.0).abs() < 0.01);
    }

    #[test]
    fn the_turning_age_resolves_once_the_era_advances() {
        let mut event = usurpation_event("");
        event.predicate = BetPredicate::AgeEnds;
        event.target_kind = TargetKind::World;
        event.created_era = 3;
        // Still the same age: unfulfilled.
        assert!(!event.is_satisfied(&[], &[], &[], 3));
        // A new age has dawned: satisfied.
        assert!(event.is_satisfied(&[], &[], &[], 4));
        // A near-breaking age reads far likelier to turn than a calm one.
        let calm = event.likelihood(&[], &[], &[], 0.2);
        let breaking = event.likelihood(&[], &[], &[], 0.95);
        assert!(breaking > calm && calm < 0.1);
    }
}
