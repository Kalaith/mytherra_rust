use super::*;

fn balance() -> ChampionBalance {
    crate::data::GameData::load().unwrap().balance.champion
}

#[test]
fn rank_never_decreases_and_respects_cap() {
    let b = balance();
    let mut champ = Champion::designate("h".to_owned(), ChampionFocus::Valor);
    champ.bond = 250.0;
    champ.quests = 40;
    champ.recompute_rank(&b);
    assert_eq!(champ.rank, b.rank_cap);
    champ.bond = 0.0;
    champ.quests = 0;
    champ.recompute_rank(&b);
    assert_eq!(champ.rank, b.rank_cap, "rank must not drop");
}

#[test]
fn cultivate_cost_grows_with_rank() {
    let b = balance();
    let mut champ = Champion::designate("h".to_owned(), ChampionFocus::Devotion);
    let low = champ.cultivate_cost(&b);
    champ.rank = 5;
    assert!(champ.cultivate_cost(&b) > low);
}

#[test]
fn quest_step_is_clamped() {
    let b = balance();
    let champ = Champion::designate("h".to_owned(), ChampionFocus::Wisdom);
    let step = champ.quest_step(1, &b);
    assert!(step >= b.quest.min && step <= b.quest.max);
}
