use super::*;

fn balance() -> MagicBalance {
    crate::data::GameData::load().unwrap().balance.magic
}

fn path() -> MagicPath {
    MagicPath::from_seed(&MagicPathSeed {
        id: "p".to_owned(),
        name: "P".to_owned(),
        description: String::new(),
        effect_stat: MagicStat::Prosperity,
        effect_per_tick: 0.3,
    })
}

#[test]
fn thresholds_drive_state() {
    let b = balance();
    let mut p = path();
    p.recompute_state(&b);
    assert_eq!(p.state, MagicState::Dormant);

    p.progress = b.emerging_progress;
    p.evidence = b.emerging_evidence;
    p.recompute_state(&b);
    assert_eq!(p.state, MagicState::Emerging);

    p.progress = b.known_progress;
    p.evidence = b.known_evidence;
    p.recompute_state(&b);
    assert_eq!(p.state, MagicState::Known);
}

#[test]
fn effect_scale_matches_state() {
    let b = balance();
    let mut p = path();
    assert_eq!(p.effect_scale(&b), 0.0);
    p.state = MagicState::Known;
    assert_eq!(p.effect_scale(&b), 1.0);
}
