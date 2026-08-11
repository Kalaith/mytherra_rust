use super::ConsequenceEffect;

#[test]
fn only_a_bloom_is_a_boon() {
    assert!(ConsequenceEffect::SettlementBloom(5.0).is_boon());
    assert!(!ConsequenceEffect::SettlementBlight(5.0).is_boon());
    assert!(!ConsequenceEffect::RegionUnrest {
        chaos: 1.0,
        danger: 1.0
    }
    .is_boon());
}
