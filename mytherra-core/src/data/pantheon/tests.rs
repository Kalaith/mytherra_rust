use super::PantheonStat::*;

#[test]
fn prosperity_and_magic_are_boons_chaos_and_danger_banes() {
    assert!(Prosperity.rising_is_good());
    assert!(Magic.rising_is_good());
    assert!(!Chaos.rising_is_good());
    assert!(!Danger.rising_is_good());
}
