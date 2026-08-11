use super::*;

fn balance() -> HeroBalance {
    crate::data::GameData::load().unwrap().balance.hero
}

fn hero(level: u32) -> Hero {
    Hero::from_seed(&HeroSeed {
        id: "h".to_owned(),
        name: "H".to_owned(),
        role: HeroRole::Warrior,
        region_id: "r".to_owned(),
        level,
        age: 30,
    })
}

#[test]
fn life_expectancy_grows_with_level() {
    let b = balance();
    assert!(hero(10).life_expectancy(&b) > hero(1).life_expectancy(&b));
}

#[test]
fn level_up_chance_falls_off_with_level() {
    let b = balance();
    assert!(hero(1).level_up_chance(&b) > hero(20).level_up_chance(&b));
    assert!(hero(20).level_up_chance(&b) > hero(60).level_up_chance(&b));
}

#[test]
fn peril_quickens_a_heros_growth() {
    let b = balance();
    let h = hero(5);
    assert!(
        h.level_up_chance_in(100.0, false, &b) > h.level_up_chance_in(0.0, false, &b),
        "a hero in a dangerous land should grow faster than one at peace"
    );
    assert_eq!(
        h.level_up_chance_in(0.0, false, &b),
        h.level_up_chance(&b),
        "a placid region with no kinship leaves the base growth chance untouched"
    );
}

#[test]
fn a_hero_in_their_element_grows_faster() {
    // A calm land that suits a hero's calling still quickens their growth
    // above the bare base, purely through the culture-kinship bonus (GDD 5.4).
    let b = balance();
    let h = hero(5);
    assert!(
        h.level_up_chance_in(0.0, true, &b) > h.level_up_chance_in(0.0, false, &b),
        "a hero whose calling suits the land should grow faster than one adrift"
    );
    assert!(
        (h.level_up_chance_in(0.0, true, &b)
            - h.level_up_chance(&b) * (1.0 + b.level_up.culture_match_bonus))
            .abs()
            < f32::EPSILON,
        "kinship adds exactly its bonus fraction"
    );
}

#[test]
fn renown_earns_titles_in_ascending_tiers() {
    let data = crate::data::GameData::load().unwrap();
    let titles = &data.strings.heroes.renown_titles;
    let thresholds = &data.balance.hero.renown.thresholds;
    let mut h = hero(1);

    h.renown = 0.0;
    assert_eq!(
        h.title(titles, thresholds),
        "",
        "an unknown hero has no title"
    );
    h.renown = thresholds[0];
    assert_eq!(h.title(titles, thresholds), titles[0].as_str());
    h.renown = *thresholds.last().unwrap() + 1_000.0;
    assert_eq!(
        h.title(titles, thresholds),
        titles.last().unwrap().as_str(),
        "a hero past the top threshold earns the highest title"
    );
}
