use super::*;
use crate::data::{ClimateType, Culture, GameData};

fn named(name: &str) -> Region {
    Region::from_seed(
        &RegionSeed {
            id: name.to_owned(),
            name: name.to_owned(),
            climate: ClimateType::Temperate,
            culture: Culture::Pastoral,
            prosperity: 50.0,
            chaos: 20.0,
            danger: 20.0,
            magic_affinity: 40.0,
            population: 1000.0,
            cultural_influence: 40.0,
            divine_resonance: 50.0,
        },
        &GameData::load().unwrap().balance.region,
    )
}

#[test]
fn a_region_set_on_expansion_gains_a_founding_bonus() {
    let data = GameData::load().unwrap();
    let region = named("aldervale");
    let threshold = data.balance.civilization.apply_threshold;
    let frontier = &data.balance.frontier;
    let expansion = data
        .agendas
        .iter()
        .position(|a| a.id == "expansion")
        .unwrap();

    // Boost Expansion to this region's prevailing course.
    let mut entry = crate::world::RegionAgendas::new("aldervale".to_owned(), data.agendas.len());
    entry.boosts[expansion] = 500.0;
    let civ = vec![entry];
    let bonus = expansion_bonus(&region, &civ, &data.agendas, threshold, frontier);
    assert!(
        (bonus - frontier.expansion_found_chance).abs() < f32::EPSILON,
        "an expansion-minded region should gain exactly the founding bonus"
    );

    // A region with no civilization entry gains nothing.
    assert_eq!(
        expansion_bonus(&region, &[], &data.agendas, threshold, frontier),
        0.0
    );
}

#[test]
fn resident_rangers_lend_a_founding_bonus() {
    use crate::data::{HeroRole, HeroSeed};
    let data = GameData::load().unwrap();
    let frontier = &data.balance.frontier;
    let ranger = |id: &str, region: &str, alive: bool| {
        let mut h = Hero::from_seed(&HeroSeed {
            id: id.to_owned(),
            name: id.to_owned(),
            role: HeroRole::Ranger,
            region_id: region.to_owned(),
            level: 10,
            age: 30,
        });
        h.is_alive = alive;
        h
    };
    let mut scout = ranger("scout2", "home", true);
    scout.role = HeroRole::Warrior; // a warrior is no pathfinder

    let heroes = vec![
        ranger("pathfinder", "home", true), // counts
        ranger("second", "home", true),     // counts
        ranger("distant", "away", true),    // wrong region
        ranger("fallen", "home", false),    // dead
        scout,                              // wrong role
    ];
    // Two living rangers at home -> exactly two steps of the bonus.
    assert!(
        (ranger_bonus("home", &heroes, frontier) - 2.0 * frontier.ranger_found_chance).abs()
            < f32::EPSILON,
        "two resident rangers should lend exactly two steps of the founding bonus"
    );
    assert_eq!(
        ranger_bonus("elsewhere", &heroes, frontier),
        0.0,
        "a land with no rangers gains nothing"
    );
}

#[test]
fn roman_numerals_render() {
    assert_eq!(roman(2), "II");
    assert_eq!(roman(4), "IV");
    assert_eq!(roman(9), "IX");
    assert_eq!(roman(14), "XIV");
}

#[test]
fn make_unique_appends_an_ordinal_only_on_collision() {
    let regions = vec![named("Aldermoor Frontier")];
    // A free name is left as-is.
    assert_eq!(
        make_unique("Sylvan Reach".to_owned(), &regions),
        "Sylvan Reach"
    );
    // A taken name gains an ordinal...
    assert_eq!(
        make_unique("Aldermoor Frontier".to_owned(), &regions),
        "Aldermoor Frontier II"
    );
    // ...and climbs past further collisions.
    let regions = vec![named("Aldermoor Frontier"), named("Aldermoor Frontier II")];
    assert_eq!(
        make_unique("Aldermoor Frontier".to_owned(), &regions),
        "Aldermoor Frontier III"
    );
}
