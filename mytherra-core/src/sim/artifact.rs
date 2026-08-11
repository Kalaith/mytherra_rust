//! Per-tick artifact behaviour (GDD 5.6): each relic nudges its region by its
//! focus, accrues instability, and — if never stabilized — eventually backlashes,
//! shattering and scarring its region. Deterministic: no RNG.

use crate::data::strings::ChronicleText;
use crate::data::{fill, ArtifactBalance, ArtifactFocus, HeroRole, RegionBalance};
use crate::world::{
    Artifact, Chronicle, ConsequenceEffect, DelayedConsequence, EventKind, Hero, Region,
};

/// Advance every artifact by one tick, resolving any backlashes. A backlash
/// scars its region at once and schedules a two-step aftermath chain onto
/// `pending` (GDD 5.6).
#[allow(clippy::too_many_arguments)]
pub fn tick_artifacts(
    artifacts: &mut Vec<Artifact>,
    regions: &mut [Region],
    heroes: &[Hero],
    pending: &mut Vec<DelayedConsequence>,
    balance: &ArtifactBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    let mut backlashed: Vec<(String, String)> = Vec::new();

    for artifact in artifacts.iter_mut() {
        apply_focus(artifact, regions, balance, region_balance);
        // Turbulent magic frays a relic faster: its region's chaos accelerates
        // the slide toward backlash (GDD 5.6).
        let region_chaos = regions
            .iter()
            .find(|r| r.id == artifact.region_id)
            .map(|r| r.chaos)
            .unwrap_or(0.0);
        // Arcane keepers tend the relic: every living Mage or Scholar dwelling in
        // its region understands its wild power and slows its fraying (GDD 5.6 <->
        // 5.4), so a relic in a learned land endures far longer than one abandoned
        // to the unlettered. Keepers only delay the doom, never avert it — the
        // growth is floored, so even the best-kept relic drifts to backlash in the
        // end.
        let keepers = heroes
            .iter()
            .filter(|h| {
                h.is_alive
                    && matches!(h.role, HeroRole::Mage | HeroRole::Scholar)
                    && h.region_id == artifact.region_id
            })
            .count();
        let growth = (artifact.instability_growth(region_chaos, balance)
            - keepers as f32 * balance.keeper_stability)
            .max(balance.min_instability_growth);
        artifact.instability += growth;

        if artifact.instability >= balance.backlash_threshold {
            let region_id = artifact.region_id.clone();
            let region_name = regions
                .iter_mut()
                .find(|r| r.id == region_id)
                .map(|r| {
                    r.apply_deltas(
                        0.0,
                        balance.backlash_chaos,
                        balance.backlash_danger,
                        0.0,
                        region_balance,
                    );
                    r.name.clone()
                })
                .unwrap_or_else(|| region_id.clone());
            schedule_aftermath(pending, &region_id, &artifact.name, balance);
            backlashed.push((artifact.name.clone(), region_name));
        }
    }

    artifacts.retain(|a| a.instability < balance.backlash_threshold);

    for (artifact_name, region_name) in backlashed {
        chronicle.push(
            year,
            EventKind::Region,
            fill(
                &text.artifact_backlash,
                &[("artifact", artifact_name), ("region", region_name)],
            ),
        );
    }
}

/// Apply an artifact's focus nudge to its region for this tick.
fn apply_focus(
    artifact: &Artifact,
    regions: &mut [Region],
    balance: &ArtifactBalance,
    region_balance: &RegionBalance,
) {
    let Some(region) = regions.iter_mut().find(|r| r.id == artifact.region_id) else {
        return;
    };
    // A relic bites deepest where the arcane runs strong (GDD 5.6), the same
    // attunement scaling the Magic tool uses.
    let attunement = balance.attunement_base + region.magic_affinity * balance.attunement_coeff;
    let delta = artifact.focus_delta(balance) * attunement;
    let (prosperity, chaos, danger, magic) = match artifact.focus {
        ArtifactFocus::Protection => (0.0, 0.0, delta, 0.0),
        ArtifactFocus::Prosperity => (delta, 0.0, 0.0, 0.0),
        ArtifactFocus::War => (0.0, delta, 0.0, 0.0),
        ArtifactFocus::Knowledge => (0.0, 0.0, 0.0, delta),
    };
    region.apply_deltas(prosperity, chaos, danger, magic, region_balance);
}

/// Queue the delayed steps that follow a shattering: a blighted settlement,
/// then a shockwave that shakes the region's heroes, then a later pulse of
/// regional unrest.
fn schedule_aftermath(
    pending: &mut Vec<DelayedConsequence>,
    region_id: &str,
    source: &str,
    balance: &ArtifactBalance,
) {
    pending.push(DelayedConsequence {
        region_id: region_id.to_owned(),
        source: source.to_owned(),
        delay: balance.aftermath_blight_delay,
        effect: ConsequenceEffect::SettlementBlight(balance.aftermath_blight_prosperity),
    });
    pending.push(DelayedConsequence {
        region_id: region_id.to_owned(),
        source: source.to_owned(),
        delay: balance.aftermath_hero_delay,
        effect: ConsequenceEffect::HeroesShaken(balance.aftermath_hero_renown),
    });
    pending.push(DelayedConsequence {
        region_id: region_id.to_owned(),
        source: source.to_owned(),
        delay: balance.aftermath_unrest_delay,
        effect: ConsequenceEffect::RegionUnrest {
            chaos: balance.aftermath_unrest_chaos,
            danger: balance.aftermath_unrest_danger,
        },
    });
}

#[cfg(test)]
mod tests;
