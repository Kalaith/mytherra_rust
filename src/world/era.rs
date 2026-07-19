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
            + struggling * balance.collapse_struggling,
        conquest: avg_danger * balance.conquest_danger
            + wartorn * balance.conquest_wartorn
            + conquest_momentum * balance.conquest_momentum_weight,
        rupture: avg_magic * balance.rupture_magic + known * balance.rupture_known,
        divine_war: pending_stake as f32 * balance.divinewar_stake
            + fallen * balance.divinewar_fallen
            + low_favor * balance.divinewar_lowfavor,
    }
}

/// Generate a fresh era name from the word banks.
pub fn generate_era_name(bank: &EraNameBank, rng: &mut SeededRng) -> String {
    let prefix = rng.choose(&bank.prefixes).cloned().unwrap_or_default();
    let title = rng.choose(&bank.titles).cloned().unwrap_or_default();
    format!("The {prefix} {title}")
}
