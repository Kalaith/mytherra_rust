use super::*;
use crate::data::{GameData, HeroRole};

fn hero(region: &str, level: u32, renown: f32) -> Hero {
    Hero {
        id: "h".to_owned(),
        name: "H".to_owned(),
        role: HeroRole::Warrior,
        region_id: region.to_owned(),
        level,
        age: 30,
        is_alive: true,
        renown,
    }
}

#[test]
fn a_famous_hero_defends_its_region_even_below_the_level_bar() {
    let balance = GameData::load().unwrap().balance.conquest;
    // A low-level but famous hero shields its region...
    let famous = vec![hero("aldermoor", 1, balance.defender_renown_min + 1.0)];
    assert!(has_defender(&famous, "aldermoor", &balance));
    // ...an equally-low unknown does not...
    let unknown = vec![hero("aldermoor", 1, 0.0)];
    assert!(!has_defender(&unknown, "aldermoor", &balance));
    // ...a seasoned hero shields regardless of renown...
    let veteran = vec![hero("aldermoor", balance.defender_min_level, 0.0)];
    assert!(has_defender(&veteran, "aldermoor", &balance));
    // ...and a defender guards only its own home.
    assert!(!has_defender(&famous, "kharzul", &balance));
}

#[test]
fn a_regions_course_shapes_the_conquest_margin() {
    let data = GameData::load().unwrap();
    let balance = &data.balance.conquest;
    let threshold = data.balance.civilization.apply_threshold;
    let region = crate::world::WorldState::new(&data).regions[0].clone();
    let idx = |id: &str| data.agendas.iter().position(|a| a.id == id).unwrap();
    let with_course = |course: usize| {
        let mut entry = crate::world::RegionAgendas::new(region.id.clone(), data.agendas.len());
        entry.boosts[course] = 500.0;
        agenda_margin(&region, &[entry], &data.agendas, threshold, balance)
    };

    // Defense widens the required margin; Rivalry narrows it (bolder attacks,
    // a more exposed defence); another course does neither.
    assert!(
        (with_course(idx("defense")) - balance.defense_margin_bonus).abs() < f32::EPSILON,
        "a defense course should widen the margin"
    );
    assert!(
        (with_course(idx("rivalry")) + balance.rivalry_aggression).abs() < f32::EPSILON,
        "a rivalry course should narrow the margin"
    );
    assert_eq!(with_course(idx("recovery")), 0.0);
    // A region with no civilization entry contributes nothing.
    assert_eq!(
        agenda_margin(&region, &[], &data.agendas, threshold, balance),
        0.0
    );
}
