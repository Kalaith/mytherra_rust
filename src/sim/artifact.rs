//! Per-tick artifact behaviour (GDD 5.6): each relic nudges its region by its
//! focus, accrues instability, and — if never stabilized — eventually backlashes,
//! shattering and scarring its region. Deterministic: no RNG.

use crate::data::strings::ChronicleText;
use crate::data::{fill, ArtifactBalance, ArtifactFocus, RegionBalance};
use crate::world::{Artifact, Chronicle, ConsequenceEffect, DelayedConsequence, EventKind, Region};

/// Advance every artifact by one tick, resolving any backlashes. A backlash
/// scars its region at once and schedules a two-step aftermath chain onto
/// `pending` (GDD 5.6).
#[allow(clippy::too_many_arguments)]
pub fn tick_artifacts(
    artifacts: &mut Vec<Artifact>,
    regions: &mut [Region],
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
        artifact.instability += artifact.instability_growth(balance);

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
/// then a later pulse of regional unrest.
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
        delay: balance.aftermath_unrest_delay,
        effect: ConsequenceEffect::RegionUnrest {
            chaos: balance.aftermath_unrest_chaos,
            danger: balance.aftermath_unrest_danger,
        },
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::world::WorldState;

    #[test]
    fn unstable_artifact_eventually_backlashes() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let before = world.artifacts.len();
        // Run long enough that at least one relic crosses the backlash line.
        for _ in 0..60 {
            tick_artifacts(
                &mut world.artifacts,
                &mut world.regions,
                &mut world.pending_consequences,
                &data.balance.artifact,
                &data.balance.region,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
        }
        assert!(world.artifacts.len() < before, "an artifact should shatter");
    }

    #[test]
    fn a_relic_reshapes_an_attuned_land_more_strongly() {
        // The same Prosperity relic lifts an arcane-attuned region more than a
        // barren one — a relic bites deepest where magic runs strong (GDD 5.6).
        let data = GameData::load().unwrap();
        let gain = |magic_affinity: f32| {
            let mut world = WorldState::new(&data);
            world.artifacts.clear();
            world.pending_consequences.clear();
            world.regions.truncate(1);
            world.regions[0].magic_affinity = magic_affinity;
            world.regions[0].prosperity = 50.0;
            world.artifacts.push(Artifact {
                id: "relic".to_owned(),
                name: "Test Relic".to_owned(),
                focus: ArtifactFocus::Prosperity,
                power: 4,
                instability: 0.0,
                region_id: world.regions[0].id.clone(),
            });
            tick_artifacts(
                &mut world.artifacts,
                &mut world.regions,
                &mut world.pending_consequences,
                &data.balance.artifact,
                &data.balance.region,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
            world.regions[0].prosperity - 50.0
        };

        assert!(
            gain(100.0) > gain(0.0),
            "a relic should reshape an attuned land more strongly than a barren one"
        );
    }
}
