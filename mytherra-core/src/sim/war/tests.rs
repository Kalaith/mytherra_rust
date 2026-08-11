use super::*;
use crate::data::{GameData, HeroRole, HeroSeed};
use crate::world::WorldState;

fn warrior(id: &str, region_id: &str, level: u32) -> Hero {
    Hero::from_seed(&HeroSeed {
        id: id.to_owned(),
        name: id.to_owned(),
        role: HeroRole::Warrior,
        region_id: region_id.to_owned(),
        level,
        age: 30,
    })
}

fn run(world: &mut WorldState, data: &GameData, balance: &WarBalance) {
    tick_wars(
        &mut world.wars,
        &mut world.regions,
        &mut world.settlements,
        &world.heroes,
        &world.artifacts,
        &world.pacts,
        &world.vassalages,
        &mut world.war_seq,
        balance,
        &data.balance.region,
        &mut world.rng,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
}

#[test]
fn a_belligerent_land_makes_war_on_the_realms_richest() {
    // A region seething with chaos and danger declares war on the wealthiest
    // other region, not on a poorer one (GDD 5.2).
    let data = GameData::load().unwrap();
    let mut balance = data.balance.war.clone();
    balance.ignite_chance = 1.0; // certain this tick
    let mut world = WorldState::new(&data);
    world.wars.clear();
    // Region 0 is belligerent; region 2 is the richest of the rest.
    world.regions[0].chaos = 90.0;
    world.regions[0].danger = 90.0;
    for (i, r) in world.regions.iter_mut().enumerate() {
        r.prosperity = if i == 2 { 95.0 } else { 40.0 };
    }
    let aggressor = world.regions[0].id.clone();
    let richest = world.regions[2].id.clone();

    run(&mut world, &data, &balance);

    assert_eq!(world.wars.len(), 1, "a war should be declared");
    assert_eq!(world.wars[0].aggressor_id, aggressor);
    assert_eq!(
        world.wars[0].defender_id, richest,
        "the belligerent should strike at the realm's richest"
    );
}

#[test]
fn one_does_not_make_war_on_a_sworn_ally() {
    // The belligerent would strike the richest region — but an alliance with it
    // stays its hand, and it falls on the next-richest instead (GDD 5.2).
    use crate::world::Pact;
    let data = GameData::load().unwrap();
    let mut balance = data.balance.war.clone();
    balance.ignite_chance = 1.0;
    let mut world = WorldState::new(&data);
    world.wars.clear();
    world.regions[0].chaos = 90.0;
    world.regions[0].danger = 90.0;
    // Region 2 richest, region 3 next-richest.
    for (i, r) in world.regions.iter_mut().enumerate() {
        r.prosperity = match i {
            2 => 95.0,
            3 => 80.0,
            _ => 40.0,
        };
    }
    let aggressor = world.regions[0].id.clone();
    let richest = world.regions[2].id.clone();
    let next_richest = world.regions[3].id.clone();
    // The aggressor is sworn to the richest.
    world.pacts.push(Pact {
        id: "p".to_owned(),
        region_a: aggressor.clone(),
        region_b: richest.clone(),
        age: 2,
    });

    run(&mut world, &data, &balance);

    assert_eq!(world.wars.len(), 1, "a war should still be declared");
    assert_eq!(
        world.wars[0].defender_id, next_richest,
        "war should fall on the next-richest, not the sworn ally"
    );
}

#[test]
fn one_does_not_make_war_on_a_vassal_or_overlord() {
    // A vassalage bond stays the sword as an alliance does: the belligerent
    // would strike the richest region, but holding it as a vassal spares it, and
    // war falls on the next-richest instead (GDD 5.2).
    use crate::world::Vassalage;
    let data = GameData::load().unwrap();
    let mut balance = data.balance.war.clone();
    balance.ignite_chance = 1.0;
    let mut world = WorldState::new(&data);
    world.wars.clear();
    world.regions[0].chaos = 90.0;
    world.regions[0].danger = 90.0;
    for (i, r) in world.regions.iter_mut().enumerate() {
        r.prosperity = match i {
            2 => 95.0,
            3 => 80.0,
            _ => 40.0,
        };
    }
    let aggressor = world.regions[0].id.clone();
    let richest = world.regions[2].id.clone();
    let next_richest = world.regions[3].id.clone();
    // The belligerent holds the richest region as its vassal.
    world.vassalages.push(Vassalage {
        id: "v".to_owned(),
        overlord_id: aggressor,
        vassal_id: richest,
        age: 2,
    });

    run(&mut world, &data, &balance);

    assert_eq!(world.wars.len(), 1, "a war should still be declared");
    assert_eq!(
        world.wars[0].defender_id, next_richest,
        "an overlord does not war the vassal it protects"
    );
}

#[test]
fn a_settled_realm_stays_at_peace() {
    // Below the belligerence threshold, no war is declared however lucky the
    // roll.
    let data = GameData::load().unwrap();
    let mut balance = data.balance.war.clone();
    balance.ignite_chance = 1.0;
    let mut world = WorldState::new(&data);
    world.wars.clear();
    for r in &mut world.regions {
        r.chaos = 20.0;
        r.danger = 20.0;
    }
    run(&mut world, &data, &balance);
    assert!(world.wars.is_empty(), "a calm realm makes no war");
}

#[test]
fn war_drains_both_combatants() {
    let data = GameData::load().unwrap();
    let mut balance = data.balance.war.clone();
    balance.ignite_chance = 0.0; // study the war we plant
    let mut world = WorldState::new(&data);
    let a = world.regions[0].id.clone();
    let b = world.regions[1].id.clone();
    world.regions[0].prosperity = 60.0;
    world.regions[1].prosperity = 60.0;
    let (pa, pb) = (world.regions[0].prosperity, world.regions[1].prosperity);
    let (da, db) = (world.regions[0].danger, world.regions[1].danger);
    world.wars.push(War {
        id: "w".to_owned(),
        aggressor_id: a,
        defender_id: b,
        intensity: 1.0,
        age: 0,
    });

    run(&mut world, &data, &balance);

    assert!(
        world.regions[0].prosperity < pa && world.regions[1].prosperity < pb,
        "war should drain both sides' prosperity"
    );
    assert!(
        world.regions[0].danger > da && world.regions[1].danger > db,
        "war should raise both sides' peril"
    );
}

#[test]
fn the_mightier_side_prevails_and_scars_the_loser() {
    // A war between a martially strong aggressor and a weak defender ends with
    // the strong prevailing and the weak scarred (GDD 5.2).
    let data = GameData::load().unwrap();
    let mut balance = data.balance.war.clone();
    balance.ignite_chance = 0.0;
    balance.intensity_decay = 1.0; // burn out and resolve this tick
    let mut world = WorldState::new(&data);
    let strong = world.regions[0].id.clone();
    let weak = world.regions[1].id.clone();
    world
        .heroes
        .retain(|h| h.region_id != strong && h.region_id != weak);
    world.heroes.push(warrior("host", &strong, 40)); // strong host
    world.regions[1].prosperity = 60.0;
    let weak_prosperity_before = world.regions[1].prosperity;
    world.wars.push(War {
        id: "w".to_owned(),
        aggressor_id: strong.clone(),
        defender_id: weak.clone(),
        intensity: balance.min_intensity, // already at the floor; decays out
        age: 5,
    });

    run(&mut world, &data, &balance);

    assert!(world.wars.is_empty(), "the war should be resolved");
    assert!(
        world.regions[1].prosperity < weak_prosperity_before,
        "the defeated side should be scarred"
    );
    assert!(
        world
            .chronicle
            .iter_newest()
            .any(|e| e.message.contains("prevails")),
        "a decisive victory should be chronicled"
    );
}

#[test]
fn a_war_relic_wins_a_war_that_would_have_been_lost() {
    // A region outmatched in the field is carried to victory by a mighty War
    // relic bound to it, so the same war it would have lost, it wins (GDD 5.2
    // <-> 5.6).
    use crate::world::Artifact;
    let data = GameData::load().unwrap();
    let mut balance = data.balance.war.clone();
    balance.ignite_chance = 0.0;
    balance.intensity_decay = 1.0; // resolve this tick
    let region_id = |w: &WorldState, i: usize| w.regions[i].id.clone();

    // The setup: a strong host in region B, a lone weak defender in region A.
    // Without a relic, A loses; with one, A wins.
    let outcome = |with_relic: bool| {
        let mut world = WorldState::new(&data);
        let a = region_id(&world, 0);
        let b = region_id(&world, 1);
        world
            .heroes
            .retain(|h| h.region_id != a && h.region_id != b);
        world.heroes.push(warrior("scout", &a, 3)); // A is weak
        world.heroes.push(warrior("host", &b, 30)); // B is strong
        world.artifacts.clear();
        if with_relic {
            world.artifacts.push(Artifact {
                id: "warblade".to_owned(),
                name: "The Warblade".to_owned(),
                focus: crate::data::ArtifactFocus::War,
                power: 9,
                instability: 0.0,
                region_id: a.clone(),
            });
        }
        world.regions[0].prosperity = 60.0;
        world.wars.push(War {
            id: "w".to_owned(),
            aggressor_id: a.clone(),
            defender_id: b.clone(),
            intensity: balance.min_intensity,
            age: 5,
        });
        let before = world.regions[0].prosperity;
        run(&mut world, &data, &balance);
        // A was scarred (lost) if its prosperity dropped by the loser scar.
        world.regions[0].prosperity < before - 1.0
    };

    assert!(
        outcome(false),
        "without a relic, the weak region should lose and be scarred"
    );
    assert!(
        !outcome(true),
        "a War relic should carry the weak region to victory, sparing it the scar"
    );
}

#[test]
fn a_strong_ally_turns_a_war_a_land_would_have_lost() {
    // A weak region loses its war alone, but a sworn ally sending its own host
    // to the defence carries it to victory instead (GDD 5.2).
    use crate::world::Pact;
    let data = GameData::load().unwrap();
    let mut balance = data.balance.war.clone();
    balance.ignite_chance = 0.0;
    balance.intensity_decay = 1.0; // resolve this tick

    // Region A (weak) is attacked by region B (strong). Region C is A's ally.
    let scarred = |with_ally: bool| {
        let mut world = WorldState::new(&data);
        let a = world.regions[0].id.clone();
        let b = world.regions[1].id.clone();
        let c = world.regions[2].id.clone();
        world
            .heroes
            .retain(|h| h.region_id != a && h.region_id != b && h.region_id != c);
        world.heroes.push(warrior("scout", &a, 3)); // A weak
        world.heroes.push(warrior("host", &b, 30)); // B strong
        world.heroes.push(warrior("kin", &c, 40)); // C mighty
        world.artifacts.clear(); // no seeded war relics to skew the mights
        world.pacts.clear();
        if with_ally {
            world.pacts.push(Pact {
                id: "p".to_owned(),
                region_a: a.clone(),
                region_b: c.clone(),
                age: 3,
            });
        }
        world.regions[0].prosperity = 60.0;
        let before = world.regions[0].prosperity;
        world.wars.push(War {
            id: "w".to_owned(),
            aggressor_id: b,
            defender_id: a,
            intensity: balance.min_intensity,
            age: 5,
        });
        run(&mut world, &data, &balance);
        world.regions[0].prosperity < before - 1.0 // A was scarred (lost)
    };

    assert!(
        scarred(false),
        "alone, the weak region loses and is scarred"
    );
    assert!(
        !scarred(true),
        "a mighty ally's aid should carry the weak region to victory"
    );
}
