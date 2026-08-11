use super::*;
use crate::data::{ChampionFocus, GameData};
use crate::world::WorldState;

#[test]
fn a_champions_completed_quest_earns_its_hero_renown() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let hero_id = world.heroes[0].id.clone();
    let before = world.heroes[0].renown;

    let mut champion = Champion::designate(hero_id.clone(), ChampionFocus::Valor);
    champion.quest_progress = data.balance.champion.quest.goal; // completes this tick
    let mut champions = vec![champion];

    tick_champions(
        &mut champions,
        &mut world.heroes,
        &mut world.regions,
        &data.balance.champion,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    assert_eq!(champions[0].quests, 1);
    let after = world
        .heroes
        .iter()
        .find(|h| h.id == hero_id)
        .unwrap()
        .renown;
    assert!(
        (after - before - data.balance.champion.renown_per_quest).abs() < 0.001,
        "a completed quest should grant exactly renown_per_quest"
    );
}

#[test]
fn a_champion_is_retired_when_its_hero_dies() {
    // A champion whose hero has fallen is dropped from the roster (freeing a
    // slot for a successor) and its passing is chronicled (GDD 5.4).
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let hero_id = world.heroes[0].id.clone();
    let hero_name = world.heroes[0].name.clone();
    let mut champions = vec![Champion::designate(hero_id, ChampionFocus::Valor)];

    world.heroes[0].is_alive = false; // the hero falls

    tick_champions(
        &mut champions,
        &mut world.heroes,
        &mut world.regions,
        &data.balance.champion,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    assert!(
        champions.is_empty(),
        "a dead hero's champion should be retired, freeing the roster slot"
    );
    assert!(
        world
            .chronicle
            .iter_newest()
            .any(|e| e.message.contains(&hero_name)),
        "the champion's passing should be chronicled"
    );
}

#[test]
fn a_champion_passively_guards_its_home() {
    // A cultivated Valor champion holds back its region's danger every tick,
    // even without completing a quest (GDD 5.4).
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let hero = world.heroes[0].clone();
    let region_idx = world
        .regions
        .iter()
        .position(|r| r.id == hero.region_id)
        .unwrap();
    world.regions[region_idx].danger = 60.0;
    let danger_before = world.regions[region_idx].danger;

    let mut champion = Champion::designate(hero.id.clone(), ChampionFocus::Valor);
    champion.rank = 5; // deeply cultivated
    champion.quest_progress = 0.0; // nowhere near completing a quest this tick
    let mut champions = vec![champion];

    tick_champions(
        &mut champions,
        &mut world.heroes,
        &mut world.regions,
        &data.balance.champion,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    assert_eq!(
        champions[0].quests, 0,
        "the champion completed no quest this tick"
    );
    assert!(
        world.regions[region_idx].danger < danger_before,
        "a Valor champion's presence should still hold back danger"
    );
}

#[test]
fn a_focus_that_suits_the_hero_shapes_the_land_more() {
    use crate::data::HeroRole;
    // A Valor champion eases danger; on a warrior — whom Valor suits — it
    // should ease more than the same champion on a scholar, by exactly the
    // synergy bonus (GDD 5.4).
    let data = GameData::load().unwrap();
    let drop = |role: HeroRole| {
        let mut world = WorldState::new(&data);
        world.heroes[0].role = role;
        world.heroes[0].is_alive = true;
        let hero = world.heroes[0].clone();
        let ri = world
            .regions
            .iter()
            .position(|r| r.id == hero.region_id)
            .unwrap();
        world.regions[ri].danger = 60.0;
        let before = world.regions[ri].danger;
        let mut champion = Champion::designate(hero.id.clone(), ChampionFocus::Valor);
        champion.rank = 5;
        champion.quest_progress = 0.0; // no quest resolves this tick
        let mut champions = vec![champion];
        tick_champions(
            &mut champions,
            &mut world.heroes,
            &mut world.regions,
            &data.balance.champion,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        before - world.regions[ri].danger
    };

    let suited = drop(HeroRole::Warrior);
    let unsuited = drop(HeroRole::Scholar);
    assert!(
        suited > unsuited,
        "a focus suited to the hero should shape the land more ({suited} vs {unsuited})"
    );
    let bonus = data.balance.champion.focus_synergy_bonus;
    assert!(
        (suited - unsuited * (1.0 + bonus)).abs() < 1e-3,
        "the suited effect should be greater by exactly the synergy bonus"
    );
}

#[test]
fn a_champion_holds_its_homeland_together() {
    // A champion passively bleeds its region's secession pressure every tick,
    // without completing a quest — a standing guard against fracture (GDD 5.4).
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let hero = world.heroes[0].clone();
    let region_idx = world
        .regions
        .iter()
        .position(|r| r.id == hero.region_id)
        .unwrap();
    world.regions[region_idx].strife = 40.0;
    let strife_before = world.regions[region_idx].strife;

    let mut champion = Champion::designate(hero.id.clone(), ChampionFocus::Valor);
    champion.rank = 5;
    champion.quest_progress = 0.0; // no quest resolves this tick
    let mut champions = vec![champion];

    tick_champions(
        &mut champions,
        &mut world.heroes,
        &mut world.regions,
        &data.balance.champion,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    assert_eq!(champions[0].quests, 0, "no quest resolved this tick");
    assert!(
        world.regions[region_idx].strife < strife_before,
        "a champion's presence should bleed off secession pressure"
    );
}

#[test]
fn strong_champion_calms_its_region() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    // A maxed champion on the calmest region should resolve, not escalate.
    let hero = world.heroes[0].clone();
    let mut champion = Champion::designate(hero.id.clone(), ChampionFocus::Valor);
    champion.bond = 300.0;
    champion.rank = 10;
    champion.quest_progress = data.balance.champion.quest.goal;
    let mut champions = vec![champion];

    let region_idx = world
        .regions
        .iter()
        .position(|r| r.id == hero.region_id)
        .unwrap();
    let danger_before = world.regions[region_idx].danger;

    tick_champions(
        &mut champions,
        &mut world.heroes,
        &mut world.regions,
        &data.balance.champion,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    assert_eq!(champions[0].quests, 1);
    assert!(world.regions[region_idx].danger <= danger_before);
}

#[test]
fn focus_shapes_the_resolution_effect() {
    // A Wisdom champion resolving a rivalry kindles its region's magic — an
    // effect a Valor champion (which instead cuts danger) never produces.
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let hero = world.heroes[0].clone();
    let region_idx = world
        .regions
        .iter()
        .position(|r| r.id == hero.region_id)
        .unwrap();
    let magic_before = world.regions[region_idx].magic_affinity;

    let mut champion = Champion::designate(hero.id.clone(), ChampionFocus::Wisdom);
    champion.bond = 300.0;
    champion.rank = 10;
    champion.quest_progress = data.balance.champion.quest.goal;
    let mut champions = vec![champion];

    tick_champions(
        &mut champions,
        &mut world.heroes,
        &mut world.regions,
        &data.balance.champion,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    assert!(
        world.regions[region_idx].magic_affinity > magic_before,
        "wisdom focus should kindle magic on a resolved rivalry"
    );
}

#[test]
fn a_resolving_champion_bleeds_secession_pressure() {
    // A strong champion holding a region should quell not just the rivalry
    // but the strife feeding the genesis fracture system (GDD 5.4 ↔ 5.2).
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let hero = world.heroes[0].clone();
    let region_idx = world
        .regions
        .iter()
        .position(|r| r.id == hero.region_id)
        .unwrap();
    world.regions[region_idx].strife = 60.0;

    let mut champion = Champion::designate(hero.id.clone(), ChampionFocus::Devotion);
    champion.bond = 300.0;
    champion.rank = 10;
    champion.quest_progress = data.balance.champion.quest.goal;
    let mut champions = vec![champion];

    tick_champions(
        &mut champions,
        &mut world.heroes,
        &mut world.regions,
        &data.balance.champion,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    assert!(
        world.regions[region_idx].strife < 60.0,
        "a resolved rivalry should bleed secession pressure"
    );
}

#[test]
fn a_routed_champion_frays_its_bond() {
    // A modest champion sent against an overwhelming region is defeated, and
    // pays for it: the bond the player cultivated frays (GDD 5.4).
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let hero = world.heroes[0].clone();
    let region_idx = world
        .regions
        .iter()
        .position(|r| r.id == hero.region_id)
        .unwrap();
    world.regions[region_idx].danger = 100.0;
    world.regions[region_idx].chaos = 100.0;
    world.regions[region_idx].strife = 100.0;

    let mut champion = Champion::designate(hero.id.clone(), ChampionFocus::Valor);
    champion.bond = 50.0;
    champion.quest_progress = data.balance.champion.quest.goal;
    let mut champions = vec![champion];

    tick_champions(
        &mut champions,
        &mut world.heroes,
        &mut world.regions,
        &data.balance.champion,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    assert_eq!(champions[0].quests, 1);
    assert!(
        champions[0].bond < 50.0,
        "a routed champion should fray its bond"
    );
    assert!(champions[0].bond >= 0.0, "bond never goes negative");
}

#[test]
fn a_harder_won_triumph_forges_more_renown() {
    // Two triumphs by the same champion, differing only in the region's
    // threat: quelling the dangerous land forges more renown (GDD 5.4).
    let data = GameData::load().unwrap();
    let renown_gained = |danger: f32, chaos: f32| {
        let mut world = WorldState::new(&data);
        let hero = world.heroes[0].clone();
        let region_idx = world
            .regions
            .iter()
            .position(|r| r.id == hero.region_id)
            .unwrap();
        world.regions[region_idx].danger = danger;
        world.regions[region_idx].chaos = chaos;
        let before = world.heroes[0].renown;

        let mut champion = Champion::designate(hero.id.clone(), ChampionFocus::Valor);
        champion.bond = 500.0; // strong enough to triumph in both regions
        champion.rank = 10;
        champion.quest_progress = data.balance.champion.quest.goal;
        let mut champions = vec![champion];

        tick_champions(
            &mut champions,
            &mut world.heroes,
            &mut world.regions,
            &data.balance.champion,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        assert_eq!(champions[0].quests, 1);
        world.heroes[0].renown - before
    };

    let calm = renown_gained(10.0, 10.0);
    let dangerous = renown_gained(80.0, 80.0);
    assert!(
        dangerous > calm,
        "quelling a dangerous region should forge more renown than a quiet one"
    );
}
