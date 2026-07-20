//! Runtime era state (GDD 5.7): the current era, its accumulated pressure from
//! five weighted triggers, and the chronicle of past eras.

use crate::data::{EraBalance, EraNameBank, EraTrigger};
use crate::world::{Hero, MagicPath, MagicState, Region, RegionStatus};
use macroquad_toolkit::rng::SeededRng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EraState {
    pub number: u32,
    pub name: String,
    pub start_year: u32,
    pub dominant_trigger: EraTrigger,
    pub pressure: f32,
}

/// A closed era, kept for the Eras chronicle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EraRecord {
    pub number: u32,
    pub name: String,
    pub start_year: u32,
    pub end_year: u32,
    pub trigger: EraTrigger,
    pub pressure: f32,
    /// The human toll of the age's ending (GDD 5.7): heroes who fell in the
    /// transition and heirs who rose to meet the new age. `serde(default)` keeps
    /// older saves loadable.
    #[serde(default)]
    pub heroes_lost: u32,
    #[serde(default)]
    pub heroes_risen: u32,
    /// Wonders thrown down as the age ended (GDD 5.7 <-> 5.2).
    #[serde(default)]
    pub wonders_razed: u32,
}

/// The five trigger scores, highest of which is the era pressure (GDD 5.7).
#[derive(Debug, Clone, Copy)]
pub struct EraScores {
    pub cataclysm: f32,
    pub collapse: f32,
    pub conquest: f32,
    pub rupture: f32,
    pub divine_war: f32,
}

impl EraScores {
    /// The five (trigger, score) pairs, for display.
    pub fn all(&self) -> [(EraTrigger, f32); 5] {
        [
            (EraTrigger::Cataclysm, self.cataclysm),
            (EraTrigger::Collapse, self.collapse),
            (EraTrigger::Conquest, self.conquest),
            (EraTrigger::MagicalRupture, self.rupture),
            (EraTrigger::DivineWar, self.divine_war),
        ]
    }

    /// The dominant (highest-scoring) trigger and its score.
    pub fn dominant(&self) -> (EraTrigger, f32) {
        self.all()
            .into_iter()
            .fold((EraTrigger::Cataclysm, f32::MIN), |best, pair| {
                if pair.1 > best.1 {
                    pair
                } else {
                    best
                }
            })
    }
}

/// Compute the five era triggers from world and player state (GDD 5.7).
#[allow(clippy::too_many_arguments)]
pub fn compute_scores(
    regions: &[Region],
    heroes: &[Hero],
    magic_paths: &[MagicPath],
    favor: i64,
    max_favor: i64,
    pending_stake: i64,
    conquest_momentum: f32,
    secession_momentum: f32,
    balance: &EraBalance,
) -> EraScores {
    let n = regions.len().max(1) as f32;
    let avg_prosperity = regions.iter().map(|r| r.prosperity).sum::<f32>() / n;
    let avg_chaos = regions.iter().map(|r| r.chaos).sum::<f32>() / n;
    let avg_danger = regions.iter().map(|r| r.danger).sum::<f32>() / n;
    let avg_magic = regions.iter().map(|r| r.magic_affinity).sum::<f32>() / n;

    let ratio = |count: usize| count as f32 / n;
    let crisis = ratio(regions.iter().filter(|r| r.status.is_crisis()).count());
    let struggling = ratio(
        regions
            .iter()
            .filter(|r| r.status == RegionStatus::Struggling)
            .count(),
    );
    let wartorn = ratio(
        regions
            .iter()
            .filter(|r| r.status == RegionStatus::WarTorn)
            .count(),
    );

    let known = if magic_paths.is_empty() {
        0.0
    } else {
        magic_paths
            .iter()
            .filter(|p| p.state == MagicState::Known)
            .count() as f32
            / magic_paths.len() as f32
    };
    let fallen = if heroes.is_empty() {
        0.0
    } else {
        heroes.iter().filter(|h| !h.is_alive).count() as f32 / heroes.len() as f32
    };
    let low_favor = (1.0 - favor as f32 / max_favor.max(1) as f32).clamp(0.0, 1.0);

    EraScores {
        cataclysm: avg_danger * balance.cataclysm_danger
            + avg_chaos * balance.cataclysm_chaos
            + crisis * balance.cataclysm_crisis,
        collapse: (100.0 - avg_prosperity) * balance.collapse_prosperity
            + struggling * balance.collapse_struggling
            + secession_momentum * balance.collapse_momentum_weight,
        conquest: avg_danger * balance.conquest_danger
            + wartorn * balance.conquest_wartorn
            + conquest_momentum * balance.conquest_momentum_weight,
        rupture: avg_magic * balance.rupture_magic + known * balance.rupture_known,
        divine_war: pending_stake as f32 * balance.divinewar_stake
            + fallen * balance.divinewar_fallen
            + low_favor * balance.divinewar_lowfavor,
    }
}

/// Generate a fresh era name by drawing a prefix, a title, and one of the
/// name patterns from the banks, then filling the pattern's `{prefix}`/`{title}`
/// slots. Draw order (and count) is fixed so world generation stays
/// deterministic. When `birthed_by` is `Some`, the prefix is drawn from that
/// trigger's pool so the age is named after the cataclysm that ended the last
/// one — "The Ashen Dawn" after a Cataclysm; the first age passes `None`.
pub fn generate_era_name(
    bank: &EraNameBank,
    birthed_by: Option<EraTrigger>,
    rng: &mut SeededRng,
) -> String {
    let pool = birthed_by
        .map(|t| bank.trigger_prefixes.get(t))
        .filter(|p| !p.is_empty())
        .unwrap_or(&bank.prefixes);
    let prefix = rng.choose(pool).cloned().unwrap_or_default();
    let title = rng.choose(&bank.titles).cloned().unwrap_or_default();
    let pattern = rng
        .choose(&bank.patterns)
        .cloned()
        .unwrap_or_else(|| "The {prefix} {title}".to_string());
    crate::data::fill(&pattern, &[("prefix", prefix), ("title", title)])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bank() -> EraNameBank {
        EraNameBank {
            prefixes: vec!["Golden".into(), "Ashen".into()],
            titles: vec!["Dawn".into(), "Ruin".into()],
            patterns: vec!["The Age of {title}".into(), "The {prefix} {title}".into()],
            descendant_titles: vec![],
            trigger_prefixes: Default::default(),
        }
    }

    #[test]
    fn era_name_is_deterministic_for_a_seed() {
        let b = bank();
        let mut lhs = SeededRng::new(42);
        let mut rhs = SeededRng::new(42);
        assert_eq!(
            generate_era_name(&b, None, &mut lhs),
            generate_era_name(&b, None, &mut rhs)
        );
    }

    #[test]
    fn era_name_fills_every_slot_from_the_pools() {
        let b = bank();
        let mut rng = SeededRng::new(7);
        for _ in 0..50 {
            let name = generate_era_name(&b, None, &mut rng);
            assert!(!name.contains('{'), "left an unfilled slot: {name}");
            assert!(
                b.titles.iter().any(|t| name.contains(t)),
                "name should carry a title: {name}"
            );
        }
    }

    #[test]
    fn empty_patterns_fall_back_to_the_classic_form() {
        let mut b = bank();
        b.patterns.clear();
        let mut rng = SeededRng::new(3);
        let name = generate_era_name(&b, None, &mut rng);
        assert!(
            name.starts_with("The "),
            "expected classic form, got: {name}"
        );
    }

    #[test]
    fn an_age_is_named_after_the_trigger_that_birthed_it() {
        // A bank whose trigger pool for Cataclysm is a single unique word, so the
        // birthing trigger is unmistakable in the name; Collapse's pool is empty
        // and must fall back to the generic prefix.
        let mut b = EraNameBank {
            prefixes: vec!["Generic".into()],
            titles: vec!["Age".into()],
            patterns: vec!["The {prefix} {title}".into()],
            descendant_titles: vec![],
            trigger_prefixes: Default::default(),
        };
        b.trigger_prefixes.cataclysm = vec!["Cataclysmic".into()];
        let mut rng = SeededRng::new(11);
        assert_eq!(
            generate_era_name(&b, Some(EraTrigger::Cataclysm), &mut rng),
            "The Cataclysmic Age",
            "a cataclysm-born age draws its prefix from the cataclysm pool"
        );
        assert_eq!(
            generate_era_name(&b, Some(EraTrigger::Collapse), &mut rng),
            "The Generic Age",
            "an empty trigger pool falls back to the generic prefixes"
        );
        assert_eq!(
            generate_era_name(&b, None, &mut rng),
            "The Generic Age",
            "the first age, birthed by no trigger, uses the generic prefixes"
        );
    }
}
