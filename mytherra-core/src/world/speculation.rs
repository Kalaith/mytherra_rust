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
    /// For a `HeroChangesRegion` wager, the region the target hero dwelt in when
    /// it opened; the proposition is met once the hero lives somewhere else. Empty
    /// for every other predicate. `serde(default)` keeps old saves loadable.
    #[serde(default)]
    pub origin_region_id: String,
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
            // Reaching the age settles the wager even if the hero later dies —
            // their age freezes at death, so a hero who fell short never clears it.
            BetPredicate::HeroSurvivesToAge => self
                .hero(heroes)
                .map(|h| h.age as f32 >= self.threshold)
                .unwrap_or(false),
            // Met once the hero dwells somewhere other than the region they were
            // in when the wager opened (the origin is empty for other predicates).
            BetPredicate::HeroChangesRegion => self
                .hero(heroes)
                .map(|h| !self.origin_region_id.is_empty() && h.region_id != self.origin_region_id)
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
            // A predicate this build doesn't understand can't be judged satisfied.
            BetPredicate::Unknown => false,
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
            // Already past the age is certain; otherwise how near the hero has
            // drawn to it (a hero who died short can never clear the bar).
            BetPredicate::HeroSurvivesToAge => self
                .hero(heroes)
                .map(|h| {
                    if h.age as f32 >= self.threshold {
                        1.0
                    } else if !h.is_alive {
                        0.0
                    } else {
                        clamp01(h.age as f32 / self.threshold.max(1.0))
                    }
                })
                .unwrap_or(0.5),
            // Already departed is certain; a living hero wanders readily (GDD 5.4
            // migration), a dead one who never left never will.
            BetPredicate::HeroChangesRegion => self
                .hero(heroes)
                .map(|h| {
                    if !self.origin_region_id.is_empty() && h.region_id != self.origin_region_id {
                        1.0
                    } else if h.is_alive {
                        0.4
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
            // An unrecognised predicate has no basis to price — a coin-flip.
            BetPredicate::Unknown => 0.5,
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
mod tests;
