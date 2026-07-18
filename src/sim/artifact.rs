//! Per-tick artifact behaviour (GDD 5.6): each relic nudges its region by its
//! focus, accrues instability, and — if never stabilized — eventually backlashes,
//! shattering and scarring its region. Deterministic: no RNG.

use crate::data::strings::ChronicleText;
use crate::data::{fill, ArtifactBalance, ArtifactFocus, RegionBalance};
use crate::world::{Artifact, Chronicle, EventKind, Region};

/// Advance every artifact by one tick, resolving any backlashes.
#[allow(clippy::too_many_arguments)]
pub fn tick_artifacts(
    artifacts: &mut Vec<Artifact>,
    regions: &mut [Region],
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
            let region_name = regions
                .iter_mut()
                .find(|r| r.id == artifact.region_id)
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
                .unwrap_or_else(|| artifact.region_id.clone());
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
    let delta = artifact.focus_delta(balance);
    let (prosperity, chaos, danger, magic) = match artifact.focus {
        ArtifactFocus::Protection => (0.0, 0.0, delta, 0.0),
        ArtifactFocus::Prosperity => (delta, 0.0, 0.0, 0.0),
        ArtifactFocus::War => (0.0, delta, 0.0, 0.0),
        ArtifactFocus::Knowledge => (0.0, 0.0, 0.0, delta),
    };
    region.apply_deltas(prosperity, chaos, danger, magic, region_balance);
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
                &data.balance.artifact,
                &data.balance.region,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
        }
        assert!(world.artifacts.len() < before, "an artifact should shatter");
    }
}
