use super::*;
use crate::data::GameData;
use crate::world::WorldState;

#[test]
fn a_scheduled_consequence_fires_only_once_its_delay_elapses() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let region_id = world.regions[0].id.clone();
    let settlement_idx = world
        .settlements
        .iter()
        .position(|s| s.region_id == region_id)
        .expect("region has a settlement");
    let before = world.settlements[settlement_idx].prosperity;

    world.pending_consequences.push(DelayedConsequence {
        region_id,
        source: "The Test Relic".to_owned(),
        delay: 2,
        effect: ConsequenceEffect::SettlementBlight(-10.0),
    });

    // delay 2 -> 1, not yet due.
    tick_consequences(
        &mut world.pending_consequences,
        &mut world.regions,
        &mut world.settlements,
        &mut world.heroes,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
    assert_eq!(world.settlements[settlement_idx].prosperity, before);
    assert_eq!(world.pending_consequences.len(), 1);

    // delay 1 -> 0, fires and is removed.
    tick_consequences(
        &mut world.pending_consequences,
        &mut world.regions,
        &mut world.settlements,
        &mut world.heroes,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
    assert!(world.settlements[settlement_idx].prosperity < before);
    assert!(world.pending_consequences.is_empty());
}

#[test]
fn a_bloom_raises_the_largest_settlements_prosperity() {
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let region_id = world.regions[0].id.clone();
    // Target the region's largest settlement, and leave it room to grow.
    let idx = world
        .settlements
        .iter()
        .enumerate()
        .filter(|(_, s)| s.region_id == region_id)
        .max_by(|(_, a), (_, b)| a.population.total_cmp(&b.population))
        .map(|(i, _)| i)
        .expect("region has a settlement");
    world.settlements[idx].prosperity = 50.0;
    let before = world.settlements[idx].prosperity;

    world.pending_consequences.push(DelayedConsequence {
        region_id,
        source: "Bloomtide".to_owned(),
        delay: 1,
        effect: ConsequenceEffect::SettlementBloom(10.0),
    });
    tick_consequences(
        &mut world.pending_consequences,
        &mut world.regions,
        &mut world.settlements,
        &mut world.heroes,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
    assert!(world.settlements[idx].prosperity > before);
    assert!(world.pending_consequences.is_empty());
}

#[test]
fn a_shockwave_dims_the_legends_of_the_regions_heroes() {
    // A HeroesShaken aftermath strips renown from every living hero of the
    // shattered relic's region — and only that region — never dropping below
    // zero, and is chronicled once (GDD 5.6 <-> 5.4).
    let data = GameData::load().unwrap();
    let mut world = WorldState::new(&data);
    let region_id = world.regions[0].id.clone();
    let other_id = world.regions[1].id.clone();

    // A renowned local hero, a barely-known local hero, and a bystander in
    // another region who should be untouched.
    world.heroes[0].region_id = region_id.clone();
    world.heroes[0].is_alive = true;
    world.heroes[0].renown = 30.0;
    world.heroes[1].region_id = region_id.clone();
    world.heroes[1].is_alive = true;
    world.heroes[1].renown = 5.0; // will floor at 0, not go negative
    world.heroes[2].region_id = other_id;
    world.heroes[2].is_alive = true;
    world.heroes[2].renown = 30.0;

    world.pending_consequences.push(DelayedConsequence {
        region_id: region_id.clone(),
        source: "The Test Relic".to_owned(),
        delay: 1,
        effect: ConsequenceEffect::HeroesShaken(12.0),
    });
    tick_consequences(
        &mut world.pending_consequences,
        &mut world.regions,
        &mut world.settlements,
        &mut world.heroes,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    assert_eq!(world.heroes[0].renown, 18.0, "the renowned hero is dimmed");
    assert_eq!(world.heroes[1].renown, 0.0, "renown floors at zero");
    assert_eq!(
        world.heroes[2].renown, 30.0,
        "a hero of another region is untouched"
    );
    assert!(world.pending_consequences.is_empty());
    assert!(
        world
            .chronicle
            .iter_newest()
            .any(|e| e.message.contains("shockwave") || e.message.contains("dim")),
        "the shockwave should be chronicled"
    );
}
