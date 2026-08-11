use super::*;

fn balance() -> ArtifactBalance {
    crate::data::GameData::load().unwrap().balance.artifact
}

fn artifact(power: u32, instability: f32) -> Artifact {
    Artifact::from_seed(&ArtifactSeed {
        id: "a".to_owned(),
        name: "A".to_owned(),
        focus: ArtifactFocus::Protection,
        power,
        instability,
        region_id: "r".to_owned(),
    })
}

#[test]
fn empower_cost_grows_with_power_and_instability() {
    let b = balance();
    let low = artifact(1, 0.0).empower_cost(&b);
    assert!(artifact(5, 0.0).empower_cost(&b) > low);
    assert!(artifact(1, 90.0).empower_cost(&b) > low);
}

#[test]
fn protection_focus_reduces_danger() {
    let b = balance();
    assert!(artifact(3, 0.0).focus_delta(&b) < 0.0);
}

#[test]
fn a_transfer_unsettles_a_relic_without_instantly_shattering_it() {
    // Moving a relic must cost real instability (GDD 5.6), but a single move
    // of an already-stable relic must not push it straight past the backlash
    // line — the risk is meant to build, not be an instant kill.
    let b = balance();
    assert!(
        b.transfer_instability > 0.0,
        "a transfer should unsettle the relic"
    );
    assert!(
        b.transfer_instability < b.backlash_threshold,
        "one transfer of a fresh relic must not instantly shatter it"
    );
}
