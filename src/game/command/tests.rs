use super::*;
use mytherra_core::data::{ArtifactFocus, GameData};
use mytherra_core::world::{Artifact, PlayerState, WorldState};
use mytherra_protocol::{project, Tier};

/// A full-visibility view (Elder standing) of a fresh world — the same shape
/// the online client renders and resolves commands against.
fn elder_view() -> WorldView {
    let data = GameData::load().unwrap();
    let world = WorldState::new(&data);
    let player = PlayerState::new(&data.config);
    let elder = data.tiers.standing(Tier::Elder);
    project(&world, &player, &elder, &data).0
}

#[test]
fn next_region_for_artifact_resolves_round_robin_from_the_view() {
    let mut view = elder_view();
    assert!(
        view.regions.len() >= 2,
        "test needs at least two revealed regions"
    );
    let first = view.regions[0].id.clone();
    let second = view.regions[1].id.clone();
    // Place an artifact in the first region — the view, not the (online-stale)
    // local world, is where the resolver must find it.
    view.artifacts.push(Artifact {
        id: "test-relic".to_owned(),
        name: "Test Relic".to_owned(),
        focus: ArtifactFocus::Protection,
        power: 1,
        instability: 0.0,
        region_id: first.clone(),
    });

    // Round-robin: an artifact in regions[0] transfers to regions[1].
    assert_eq!(
        next_region_for_artifact_in(&view, "test-relic"),
        Some(second)
    );
    // From the last region it wraps back to the first.
    let last = view.regions.last().unwrap().id.clone();
    let relic = view
        .artifacts
        .iter_mut()
        .find(|a| a.id == "test-relic")
        .unwrap();
    relic.region_id = last;
    assert_eq!(
        next_region_for_artifact_in(&view, "test-relic"),
        Some(first)
    );
    // An unknown artifact resolves to nothing.
    assert_eq!(next_region_for_artifact_in(&view, "no-such"), None);
}

#[test]
fn selected_region_id_reads_and_clamps_to_the_view() {
    let view = elder_view();
    assert_eq!(selected_region_id_in(&view, 0), view.regions[0].id);
    // An out-of-range selection clamps to the last revealed region.
    let last = view.regions.last().unwrap().id.clone();
    assert_eq!(selected_region_id_in(&view, 999), last);
}
