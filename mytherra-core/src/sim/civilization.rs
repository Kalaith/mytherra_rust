//! Per-tick civilization behaviour (GDD 5.6): each region pursues its single
//! dominant agenda — the highest-scoring one that clears the threshold — which
//! nudges the region. Player boosts decay over time. Deterministic: no RNG.

use crate::data::strings::ChronicleText;
use crate::data::{fill, Agenda, CivStat, CivilizationBalance, RegionBalance};
use crate::world::{
    dominant_agenda, spillover_target, Chronicle, EventKind, Pact, Region, RegionAgendas, Vassalage,
};

/// Advance every region's agendas by one tick.
#[allow(clippy::too_many_arguments)]
pub fn tick_civilization(
    civ: &mut [RegionAgendas],
    regions: &mut [Region],
    agendas: &[Agenda],
    pacts: &[Pact],
    vassalages: &[Vassalage],
    balance: &CivilizationBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    for entry in civ.iter_mut() {
        entry.cooldown = (entry.cooldown - 1).max(0);
        for boost in entry.boosts.iter_mut() {
            *boost = (*boost - balance.boost_decay).max(0.0);
        }

        let Some(idx) = regions.iter().position(|r| r.id == entry.region_id) else {
            continue;
        };
        if let Some(a) = dominant_agenda(agendas, &regions[idx], entry, balance.apply_threshold) {
            let agenda = &agendas[a];

            // A change of prevailing course is a moment in the region's history:
            // chronicle it once when a *different* agenda takes hold (whether the
            // world's drift or the player's boost redirected it), so the
            // civilization system reads in the chronicle instead of nudging in
            // silence (GDD 5.6). Lapses back to no dominant course leave the last
            // course recorded, so re-adopting it isn't re-announced.
            if entry.current_agenda.as_deref() != Some(agenda.id.as_str()) {
                entry.current_agenda = Some(agenda.id.clone());
                chronicle.push(
                    year,
                    EventKind::Region,
                    fill(
                        &text.agenda_shift,
                        &[
                            ("region", regions[idx].name.clone()),
                            ("agenda", agenda.name.clone()),
                        ],
                    ),
                );
            }

            let (dp, dc, dd, dm) = stat_deltas(agenda.effect_stat, agenda.effect_amount);
            regions[idx].apply_deltas(dp, dc, dd, dm, region_balance);

            // An outward-facing agenda presses upon a peer — the first time
            // civilizations touch one another: a rivalrous region destabilizes
            // the neighbour it envies, an expansionist one leans on the weakest
            // (GDD 5.6).
            if agenda.spillover_amount != 0.0 {
                if let Some(target) =
                    spillover_target(regions, idx, agenda.spillover_target, pacts, vassalages)
                {
                    let (sp, sc, sd, sm) =
                        stat_deltas(agenda.spillover_stat, agenda.spillover_amount);
                    regions[target].apply_deltas(sp, sc, sd, sm, region_balance);
                }
            }
        }
    }
}

/// Map an agenda stat + amount onto (prosperity, chaos, danger, magic) deltas.
fn stat_deltas(stat: CivStat, amount: f32) -> (f32, f32, f32, f32) {
    match stat {
        CivStat::Prosperity => (amount, 0.0, 0.0, 0.0),
        CivStat::Chaos => (0.0, amount, 0.0, 0.0),
        CivStat::Danger => (0.0, 0.0, amount, 0.0),
        CivStat::Magic => (0.0, 0.0, 0.0, amount),
    }
}

#[cfg(test)]
mod tests;
