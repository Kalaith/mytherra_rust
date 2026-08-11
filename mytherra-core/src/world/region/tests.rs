use super::*;

fn seed() -> RegionSeed {
    RegionSeed {
        id: "t".to_owned(),
        name: "Test".to_owned(),
        climate: ClimateType::Temperate,
        culture: Culture::Scholarly,
        prosperity: 50.0,
        chaos: 50.0,
        danger: 50.0,
        magic_affinity: 50.0,
        population: 1000.0,
        cultural_influence: 50.0,
        divine_resonance: 50.0,
    }
}

fn even_weights(w: f32) -> HeroMightWeights {
    HeroMightWeights {
        warrior: w,
        mage: w,
        scholar: w,
        ranger: w,
        merchant: w,
        cleric: w,
    }
}

fn hero_of(region_id: &str, role: crate::data::HeroRole, level: u32, alive: bool) -> Hero {
    Hero {
        id: "h".to_owned(),
        name: "H".to_owned(),
        role,
        region_id: region_id.to_owned(),
        level,
        age: 30,
        is_alive: alive,
        renown: 0.0,
    }
}

#[test]
fn resident_might_counts_only_living_heroes_at_home() {
    use crate::data::HeroRole::Warrior;
    let heroes = vec![
        hero_of("home", Warrior, 10, true),
        hero_of("home", Warrior, 4, true), // 14 living levels at home
        hero_of("home", Warrior, 100, false), // dead: lends no might
        hero_of("away", Warrior, 50, true), // elsewhere: lends no might here
    ];
    // Warrior weight 1.0 here, so (10 + 4) * 0.5 * 1.0 = 7.0.
    let w = even_weights(1.0);
    assert_eq!(resident_might(&heroes, "home", 0.5, &w), 7.0);
    assert_eq!(resident_might(&heroes, "nowhere", 0.5, &w), 0.0);
}

#[test]
fn martial_roles_lend_more_might_than_scholarly_ones() {
    use crate::data::HeroRole::{Scholar, Warrior};
    let weights = HeroMightWeights {
        warrior: 1.0,
        scholar: 0.2,
        ..even_weights(0.5)
    };
    let warrior_land = vec![hero_of("r", Warrior, 10, true)];
    let scholar_land = vec![hero_of("r", Scholar, 10, true)];
    assert!(
        resident_might(&warrior_land, "r", 1.0, &weights)
            > resident_might(&scholar_land, "r", 1.0, &weights),
        "a warrior should lend more military might than a scholar of equal level"
    );
}

#[test]
fn pressure_drift_tracks_worsening_stats() {
    let balance = balance().region;
    let mut region = Region::from_seed(&seed(), &balance);
    // Snapshot the calm baseline, then let danger and chaos climb.
    region.prev = StatSnapshot {
        prosperity: region.prosperity,
        chaos: region.chaos,
        danger: region.danger,
        magic_affinity: region.magic_affinity,
    };
    region.danger = 90.0;
    region.chaos = 80.0;
    // Pressure now exceeds the snapshot's, so the drift the omens read is
    // positive — the age is deepening.
    assert!(region.pressure() > region.prev_pressure());
}

fn bless() -> RegionActionDef {
    RegionActionDef {
        id: "bless".to_owned(),
        name: "Bless".to_owned(),
        description: String::new(),
        cost: 15,
        prosperity: 8.0,
        chaos: -4.0,
        danger: -3.0,
        magic_affinity: 0.0,
    }
}

fn balance() -> crate::data::Balance {
    crate::data::GameData::load().unwrap().balance
}

#[test]
fn neutral_resonance_gives_base_cost_and_effect() {
    let b = balance();
    let mut region = Region::from_seed(&seed(), &b.region);
    assert_eq!(region.action_cost(&bless(), &b.region), 15);
    region.apply_action(&bless(), &b.region);
    assert!((region.prosperity - 58.0).abs() < f32::EPSILON);
    assert!((region.chaos - 46.0).abs() < f32::EPSILON);
    assert!((region.danger - 47.0).abs() < f32::EPSILON);
}

#[test]
fn a_divine_touch_consecrates_the_land() {
    // Acting on a region raises its divine resonance, so a god's repeated
    // attention makes future nudges there cheaper and stronger (GDD 5.2).
    let b = balance();
    let mut region = Region::from_seed(&seed(), &b.region);
    let before = region.divine_resonance;
    let cost_before = region.action_cost(&bless(), &b.region);
    region.apply_action(&bless(), &b.region);
    assert!(
        region.divine_resonance > before,
        "a divine act should attune the land"
    );
    // Enough repeated attention lowers the cost of acting there.
    for _ in 0..20 {
        region.apply_action(&bless(), &b.region);
    }
    assert!(
        region.action_cost(&bless(), &b.region) < cost_before,
        "a consecrated land should be cheaper to nudge"
    );
    assert!(region.divine_resonance <= 100.0, "resonance stays clamped");
}

#[test]
fn high_resonance_is_cheaper_and_stronger() {
    let b = balance();
    let mut s = seed();
    s.divine_resonance = 100.0;
    let region = Region::from_seed(&s, &b.region);
    assert!(region.action_cost(&bless(), &b.region) < 15);
    assert!(region.effect_multiplier(&b.region) > 1.0);
}

#[test]
fn stats_clamp_to_valid_range() {
    let b = balance();
    let mut s = seed();
    s.prosperity = 98.0;
    let mut region = Region::from_seed(&s, &b.region);
    for _ in 0..10 {
        region.apply_action(&bless(), &b.region);
    }
    assert!(region.prosperity <= 100.0);
    assert!(region.danger >= 0.0);
}
