//! Runtime civilization state (GDD 5.6): per-region agenda boosts and the
//! diplomacy cooldown. Agenda scores are computed live from region stats rather
//! than stored, so they always reflect the current world.

use crate::data::Agenda;
use crate::world::Region;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionAgendas {
    pub region_id: String,
    /// Player-added score boost per agenda (index-aligned with `data.agendas`);
    /// decays over time.
    pub boosts: Vec<f32>,
    /// Years until another agenda may be advanced here.
    pub cooldown: i32,
}

impl RegionAgendas {
    pub fn new(region_id: String, agenda_count: usize) -> Self {
        Self {
            region_id,
            boosts: vec![0.0; agenda_count],
            cooldown: 0,
        }
    }

    pub fn boost(&self, index: usize) -> f32 {
        self.boosts.get(index).copied().unwrap_or(0.0)
    }
}

/// An agenda's live score: a weighted-linear read of the region plus any player
/// boost (GDD 5.6).
pub fn agenda_score(agenda: &Agenda, region: &Region, boost: f32) -> f32 {
    agenda.base
        + agenda.w_prosperity * region.prosperity
        + agenda.w_chaos * region.chaos
        + agenda.w_danger * region.danger
        + agenda.w_magic * region.magic_affinity
        + agenda.w_culture * region.cultural_influence
        + boost
}

/// The single agenda a region is currently pursuing: its highest-scoring one,
/// but only if that clears the activation threshold. A society commits to one
/// prevailing course rather than every agenda at once, so a player boost that
/// makes an agenda dominant *redirects* the region (GDD 5.6). Ties break toward
/// the earliest agenda, keeping selection deterministic.
pub fn dominant_agenda(
    agendas: &[Agenda],
    region: &Region,
    entry: &RegionAgendas,
    threshold: f32,
) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    for (i, agenda) in agendas.iter().enumerate() {
        let score = agenda_score(agenda, region, entry.boost(i));
        if best.is_none_or(|(_, s)| score > s) {
            best = Some((i, score));
        }
    }
    best.filter(|&(_, score)| score >= threshold)
        .map(|(i, _)| i)
}
