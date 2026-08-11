use super::*;

#[test]
fn fill_replaces_named_placeholders() {
    let result = fill(
        "{action} on {region} ({cost} favor).",
        &[
            ("action", "Bless".to_owned()),
            ("region", "Aldermoor".to_owned()),
            ("cost", "15".to_owned()),
        ],
    );
    assert_eq!(result, "Bless on Aldermoor (15 favor).");
}
