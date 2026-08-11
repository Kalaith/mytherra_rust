use super::*;

fn balance() -> PantheonBalance {
    crate::data::GameData::load().unwrap().balance.pantheon
}

fn deity(pressure: f32) -> PantheonDeity {
    PantheonDeity::from_seed(&DeitySeed {
        id: "d".to_owned(),
        name: "D".to_owned(),
        domain: "Test".to_owned(),
        ally_id: "a".to_owned(),
        rival_id: "r".to_owned(),
        effect_stat: PantheonStat::Prosperity,
        effect_amount: 0.3,
        start_pressure: pressure,
    })
}

#[test]
fn tier_rises_with_pressure() {
    let b = balance();
    assert_eq!(deity(10.0).tier(&b), 0);
    assert!(deity(80.0).tier(&b) > deity(30.0).tier(&b));
    assert!(deity(90.0).tier_multiplier(&b) > deity(30.0).tier_multiplier(&b));
}

#[test]
fn adjust_pressure_clamps() {
    let mut deities = vec![deity(95.0)];
    deities[0].id = "x".to_owned();
    adjust_pressure(&mut deities, "x", 50.0);
    assert!(deities[0].pressure <= 100.0);
}
