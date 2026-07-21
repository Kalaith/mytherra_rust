//! Speculation events: the shared-world propositions players bet on (GDD 5.5).
//! Each event denormalizes its predicate + threshold so it can be evaluated
//! without a data lookup, and carries simulated crowd stakes so the crowd-lean
//! payout adjustment is meaningful in this local build.

use crate::data::{BetPredicate, HeroRole, TargetKind};
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
    /// Region count when the wager was opened; a `NewRegion` proposition is met
    /// once the world holds more regions than this.
    #[serde(default)]
    pub created_region_count: u32,
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
            BetPredicate::RegionResonanceAtLeast => self
                .region(regions)
                .map(|r| r.divine_resonance >= self.threshold)
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
            // The world holds more regions than when the wager opened.
            BetPredicate::NewRegion => regions.len() as u32 > self.created_region_count,
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
                        // Age, a perilous home, and frailty all sway the odds,
                        // mirroring the danger-scaled, level-mitigated death roll
                        // rather than reading age alone (GDD 5.4 <-> 5.5).
                        let danger = regions
                            .iter()
                            .find(|r| r.id == h.region_id)
                            .map(|r| r.danger)
                            .unwrap_or(0.0);
                        clamp01(h.age as f32 / 90.0 + danger / 250.0 - h.level as f32 / 100.0)
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
            BetPredicate::RegionResonanceAtLeast => self
                .region(regions)
                .map(|r| {
                    let current = clamp01(r.divine_resonance / self.threshold.max(1.0));
                    // A land served by living Clerics is being consecrated over
                    // time (GDD 5.4 <-> 5.1), so even a humble resonance trends
                    // toward the bar — the crowd prices in the devout, the way it
                    // prices in a strong defender for a conquest wager. Each
                    // resident cleric lends a little confidence, capped at 1.
                    let clerics = heroes
                        .iter()
                        .filter(|h| h.is_alive && h.role == HeroRole::Cleric && h.region_id == r.id)
                        .count();
                    clamp01(current + clerics as f32 * 0.12)
                })
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
                Some(r) => {
                    let raw = if r.status.is_crisis() { 0.4 } else { 0.05 }
                        + (100.0 - r.prosperity) / 100.0 * 0.3
                        + r.danger / 100.0 * 0.2;
                    // The crowd knows a region held by a strong, famous hero
                    // rarely falls: such a defender turns invaders back entirely
                    // (GDD 5.4 <-> 5.2), so the odds of conquest collapse when one
                    // guards it. The level/renown rule of thumb mirrors the
                    // conquest defender bars, and it rewards a player whose
                    // cultivated champion earned its home this shield.
                    let guarded = heroes.iter().any(|h| {
                        h.is_alive && h.region_id == r.id && (h.level >= 5 || h.renown >= 100.0)
                    });
                    clamp01(if guarded { raw * 0.15 } else { raw })
                }
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
            // A churning world — lands fracturing from strife or thriving toward a
            // frontier — is likelier to birth a new region.
            BetPredicate::NewRegion => {
                if regions.is_empty() {
                    0.3
                } else {
                    let churning = regions
                        .iter()
                        .filter(|r| r.strife > 30.0 || r.prosperity > 75.0)
                        .count();
                    clamp01(0.15 + churning as f32 / regions.len() as f32 * 0.6)
                }
            }
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
            created_region_count: 4,
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

    #[test]
    fn a_heros_death_odds_rise_with_peril_and_fall_with_might() {
        let mut event = usurpation_event("h");
        event.predicate = BetPredicate::HeroDies;
        event.target_kind = TargetKind::Hero;

        let mut safe = region("aldermoor");
        safe.danger = 0.0;
        let mut perilous = region("kharzul");
        perilous.danger = 100.0;
        let regions = vec![safe, perilous];

        let hero_in = |region_id: &str, level: u32| Hero {
            id: "h".to_owned(),
            name: "H".to_owned(),
            role: crate::data::HeroRole::Warrior,
            region_id: region_id.to_owned(),
            level,
            age: 40,
            is_alive: true,
            renown: 0.0,
        };
        let odds = |h: Hero| event.likelihood(&[h], &regions, &[], 0.0);

        assert!(
            odds(hero_in("kharzul", 10)) > odds(hero_in("aldermoor", 10)),
            "a hero in a war-torn land is likelier to die than one at peace"
        );
        assert!(
            odds(hero_in("kharzul", 1)) > odds(hero_in("kharzul", 40)),
            "a frail hero is likelier to die than a mighty one in the same peril"
        );
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
    fn the_crowd_prices_in_a_strong_defender() {
        // The same crisis-stricken region reads as far less likely to be
        // conquered once a strong, living hero holds it — the crowd knows a
        // guarded region rarely falls (GDD 5.5 <-> 5.4). A frail nobody does not
        // move the odds, so the crowd distinguishes a real defender.
        let event = usurpation_event("kharzul");
        let weak = vec![region("kharzul")];
        let undefended = event.likelihood(&[], &weak, &[], 0.0);

        let champion = hero("guardian", 0.0, true); // level 10, home is kharzul
        let defended = event.likelihood(&[champion], &weak, &[], 0.0);
        assert!(
            defended < undefended * 0.5,
            "a guarded region should read as far less likely to fall: {defended} vs {undefended}"
        );

        let nobody = Hero {
            level: 1,
            renown: 0.0,
            ..hero("nobody", 0.0, true)
        };
        let still_open = event.likelihood(&[nobody], &weak, &[], 0.0);
        assert!(
            (still_open - undefended).abs() < 1e-6,
            "a frail hero is no shield, so the odds are unchanged"
        );
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
    fn hallowed_ground_resolves_when_resonance_clears_the_bar() {
        // `region()` seeds divine_resonance at 50.
        let mut event = usurpation_event("kharzul");
        event.predicate = BetPredicate::RegionResonanceAtLeast;
        let regions = vec![region("kharzul")];

        event.threshold = 40.0;
        assert!(
            event.is_satisfied(&[], &regions, &[], 1),
            "50 clears a bar of 40"
        );
        event.threshold = 80.0;
        assert!(
            !event.is_satisfied(&[], &regions, &[], 1),
            "50 falls short of 80"
        );
        // With no clerics, likelihood tracks the raw ratio to the bar.
        assert!((event.likelihood(&[], &regions, &[], 0.0) - 50.0 / 80.0).abs() < 0.01);

        // The crowd reads the devout: a resident living Cleric lends confidence
        // that the land will grow hallowed, so the odds rise above the raw ratio.
        let barren = event.likelihood(&[], &regions, &[], 0.0);
        let served = event.likelihood(&[hero_cleric("kharzul", true)], &regions, &[], 0.0);
        assert!(
            served > barren,
            "a land served by a cleric should read likelier to grow hallowed"
        );
        // A fallen cleric tends no faith, so it does not move the odds.
        let fallen = event.likelihood(&[hero_cleric("kharzul", false)], &regions, &[], 0.0);
        assert!(
            (fallen - barren).abs() < 1e-6,
            "the dead consecrate nothing"
        );
    }

    fn hero_cleric(region_id: &str, alive: bool) -> Hero {
        Hero {
            id: "cleric".to_owned(),
            name: "Cleric".to_owned(),
            role: HeroRole::Cleric,
            region_id: region_id.to_owned(),
            level: 5,
            age: 40,
            is_alive: alive,
            renown: 0.0,
        }
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

    #[test]
    fn a_new_land_resolves_once_the_region_count_grows() {
        let mut event = usurpation_event("");
        event.predicate = BetPredicate::NewRegion;
        event.target_kind = TargetKind::World;
        event.created_region_count = 4;
        // Same four regions: unfulfilled.
        let four = vec![region("a"), region("b"), region("c"), region("d")];
        assert!(!event.is_satisfied(&[], &four, &[], 1));
        // A fifth region has risen: satisfied.
        let five = vec![
            region("a"),
            region("b"),
            region("c"),
            region("d"),
            region("e"),
        ];
        assert!(event.is_satisfied(&[], &five, &[], 1));
    }
}
