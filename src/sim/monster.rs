//! Per-tick bestiary (GDD 5.2): beasts emerge from perilous, untamed regions —
//! arcane horrors where magic runs strong, natural predators where the wilds are
//! merely dangerous — menace the land and raid its towns, grow into greater
//! terrors if left unopposed, and are hunted down by resident Warriors and
//! Rangers, whose mightiest earns the renown of the kill. The embodied threat
//! behind the abstract danger stat. Randomness (emergence) flows through the
//! world RNG; the menace, growth, and hunt are deterministic.

use crate::data::strings::ChronicleText;
use crate::data::{fill, HeroRole, MonsterBalance, MonsterType, RegionBalance};
use crate::world::{Chronicle, EventKind, Hero, Monster, Region, Settlement};
use macroquad_toolkit::rng::SeededRng;

/// One beast felled by a named hunter this tick: `(hero_name, beast_name,
/// region_id)`, returned so the caller can commemorate the deed in myth (GDD
/// 5.2 <-> 5.6) — the bestiary's counterpart to a hero passing into legend.
pub type BeastSlain = (String, String, String);

#[allow(clippy::too_many_arguments)]
pub fn tick_monster(
    monsters: &mut Vec<Monster>,
    regions: &mut [Region],
    settlements: &mut [Settlement],
    heroes: &mut [Hero],
    types: &[MonsterType],
    seq: &mut u64,
    balance: &MonsterBalance,
    region_balance: &RegionBalance,
    rng: &mut SeededRng,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) -> Vec<BeastSlain> {
    spawn_monsters(
        monsters, regions, types, seq, balance, rng, chronicle, text, year,
    );

    // Menace, growth, and the hunt.
    for monster in monsters.iter_mut() {
        monster.age += 1;
        let Some(ty) = types.iter().find(|t| t.id == monster.type_id) else {
            // Unknown kind (bestiary changed under an old save): let it wither.
            monster.ferocity -= balance.ferocity_growth;
            continue;
        };
        // The beast makes the land perilous...
        if let Some(region) = regions.iter_mut().find(|r| r.id == monster.region_id) {
            region.apply_deltas(
                0.0,
                0.0,
                ty.danger_per_tick * monster.ferocity,
                0.0,
                region_balance,
            );
        }
        // ...and raids the largest settlement for its people.
        if let Some(settlement) = largest_settlement(settlements, &monster.region_id) {
            let loss = settlement.population * ty.raid_population * monster.ferocity;
            settlement.population = (settlement.population - loss).max(0.0);
        }
        // The hunt: resident Warriors and Rangers grind the beast down; left
        // unopposed it grows into a greater terror.
        let might = hunter_might(heroes, &monster.region_id);
        if might > 0 {
            monster.ferocity -= might as f32 * balance.slay_per_might;
        } else {
            monster.ferocity += balance.ferocity_growth;
        }
    }

    // Beasts worn below the floor are slain (or, where no hunter remains, driven
    // off); the mightiest resident hunter claims the kill and its renown.
    let slain: Vec<(String, String)> = monsters
        .iter()
        .filter(|m| m.ferocity < balance.min_ferocity)
        .map(|m| (m.region_id.clone(), m.name.clone()))
        .collect();
    monsters.retain(|m| m.ferocity >= balance.min_ferocity);

    let mut felled: Vec<BeastSlain> = Vec::new();
    for (region_id, name) in slain {
        let slayer = heroes
            .iter_mut()
            .filter(|h| {
                h.is_alive
                    && h.region_id == region_id
                    && matches!(h.role, HeroRole::Warrior | HeroRole::Ranger)
            })
            .max_by_key(|h| h.level);
        match slayer {
            Some(hero) => {
                hero.renown += balance.slay_renown;
                let hero_name = hero.name.clone();
                chronicle.push(
                    year,
                    EventKind::Region,
                    fill(
                        &text.monster_slain,
                        &[("hero", hero_name.clone()), ("monster", name.clone())],
                    ),
                );
                felled.push((hero_name, name, region_id));
            }
            None => chronicle.push(
                year,
                EventKind::Region,
                fill(&text.monster_driven_off, &[("monster", name)]),
            ),
        }
    }
    felled
}

/// Raise fresh beasts in perilous, untamed regions that have none (GDD 5.2).
#[allow(clippy::too_many_arguments)]
fn spawn_monsters(
    monsters: &mut Vec<Monster>,
    regions: &[Region],
    types: &[MonsterType],
    seq: &mut u64,
    balance: &MonsterBalance,
    rng: &mut SeededRng,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    if types.is_empty() {
        return;
    }
    for region in regions {
        if monsters.len() >= balance.max_active {
            break;
        }
        if region.danger < balance.emergence_min_danger
            || monsters.iter().any(|m| m.region_id == region.id)
        {
            continue;
        }
        // Peril breeds beasts: the more dangerous the wilds, the likelier one
        // stalks forth.
        let chance = balance.emergence_chance + region.danger * balance.emergence_danger_coeff;
        if !rng.chance(chance) {
            continue;
        }
        // Arcane lands breed arcane horrors; the merely perilous breed predators.
        let arcane = region.magic_affinity >= balance.arcane_magic_threshold;
        let matching: Vec<&MonsterType> = types.iter().filter(|t| t.arcane == arcane).collect();
        let ty = if matching.is_empty() {
            &types[rng.below(types.len())]
        } else {
            matching[rng.below(matching.len())]
        };

        *seq += 1;
        let name = fill(
            &text.monster_name,
            &[("beast", ty.name.clone()), ("region", region.name.clone())],
        );
        monsters.push(Monster {
            id: format!("monster-{seq}"),
            name: name.clone(),
            type_id: ty.id.clone(),
            region_id: region.id.clone(),
            ferocity: ty.start_ferocity,
            age: 0,
        });
        chronicle.push(
            year,
            EventKind::Region,
            fill(&text.monster_emergence, &[("monster", name)]),
        );
    }
}

/// Combined levels of the living Warriors and Rangers dwelling in a region — the
/// might it can bring to bear against a beast.
fn hunter_might(heroes: &[Hero], region_id: &str) -> u32 {
    heroes
        .iter()
        .filter(|h| {
            h.is_alive
                && h.region_id == region_id
                && matches!(h.role, HeroRole::Warrior | HeroRole::Ranger)
        })
        .map(|h| h.level)
        .sum()
}

/// The region's most populous settlement, if any.
fn largest_settlement<'a>(
    settlements: &'a mut [Settlement],
    region_id: &str,
) -> Option<&'a mut Settlement> {
    settlements
        .iter_mut()
        .filter(|s| s.region_id == region_id)
        .max_by(|a, b| a.population.total_cmp(&b.population))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::world::WorldState;

    fn run(world: &mut WorldState, data: &GameData, balance: &MonsterBalance) -> Vec<BeastSlain> {
        tick_monster(
            &mut world.monsters,
            &mut world.regions,
            &mut world.settlements,
            &mut world.heroes,
            &data.monster_types,
            &mut world.monster_seq,
            balance,
            &data.balance.region,
            &mut world.rng,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        )
    }

    #[test]
    fn perilous_wilds_breed_more_beasts_than_safe_lands() {
        let data = GameData::load().unwrap();
        let emergences = |danger: f32| {
            let mut world = WorldState::new(&data);
            world.regions.truncate(1);
            world.regions[0].danger = danger;
            world.regions[0].magic_affinity = 0.0; // natural predators only
            let mut count = 0;
            for _ in 0..400 {
                world.monsters.clear(); // isolate emergence odds, not persistence
                run(&mut world, &data, &data.balance.monster);
                count += world.monsters.len();
            }
            count
        };
        assert!(
            emergences(90.0) > emergences(45.0),
            "the more perilous wilds should breed more beasts"
        );
    }

    #[test]
    fn a_calm_land_breeds_no_beasts() {
        // Below the danger floor, no beast emerges however unlucky the roll.
        let data = GameData::load().unwrap();
        let mut balance = data.balance.monster.clone();
        balance.emergence_chance = 1.0; // would fire every tick if eligible
        let mut world = WorldState::new(&data);
        world.regions.truncate(1);
        world.regions[0].danger = balance.emergence_min_danger - 1.0;

        run(&mut world, &data, &balance);
        assert!(
            world.monsters.is_empty(),
            "a settled, peaceful land breeds no monsters"
        );
    }

    #[test]
    fn an_arcane_land_breeds_arcane_beasts() {
        // A magic-steeped region draws only arcane horrors from the bestiary.
        let data = GameData::load().unwrap();
        let mut balance = data.balance.monster.clone();
        balance.emergence_chance = 1.0;
        let mut world = WorldState::new(&data);
        world.regions.truncate(1);
        world.regions[0].danger = balance.emergence_min_danger + 10.0;
        world.regions[0].magic_affinity = balance.arcane_magic_threshold + 10.0;
        world.monsters.clear();

        run(&mut world, &data, &balance);
        assert_eq!(world.monsters.len(), 1, "a beast should emerge");
        let type_id = &world.monsters[0].type_id;
        let ty = data
            .monster_types
            .iter()
            .find(|t| &t.id == type_id)
            .unwrap();
        assert!(
            ty.arcane,
            "a magic-steeped land should breed an arcane beast"
        );
    }

    #[test]
    fn a_beast_menaces_its_region_and_raids_its_town() {
        let data = GameData::load().unwrap();
        let mut balance = data.balance.monster.clone();
        balance.emergence_chance = 0.0; // study the beast we plant
        let mut world = WorldState::new(&data);
        let region_id = world.regions[0].id.clone();
        world.regions[0].danger = 30.0;
        // Strip any resident hunters so the beast rages unopposed.
        world.heroes.retain(|h| h.region_id != region_id);
        let sidx = world
            .settlements
            .iter()
            .enumerate()
            .filter(|(_, s)| s.region_id == region_id)
            .max_by(|(_, a), (_, b)| a.population.total_cmp(&b.population))
            .map(|(i, _)| i)
            .expect("region has a settlement");
        let pop_before = world.settlements[sidx].population;
        let danger_before = world.regions[0].danger;
        world.monsters.push(Monster {
            id: "m".to_owned(),
            name: "The Test Beast".to_owned(),
            type_id: "hill_troll".to_owned(),
            region_id,
            ferocity: 2.0,
            age: 0,
        });

        run(&mut world, &data, &balance);

        assert!(
            world.regions[0].danger > danger_before,
            "a beast should make its region more perilous"
        );
        assert!(
            world.settlements[sidx].population < pop_before,
            "a beast should raid the region's largest settlement"
        );
    }

    #[test]
    fn resident_hunters_slay_a_beast_and_the_mightiest_earns_the_renown() {
        let data = GameData::load().unwrap();
        let mut balance = data.balance.monster.clone();
        balance.emergence_chance = 0.0;
        balance.slay_per_might = 10.0; // fell it in a single tick
        let mut world = WorldState::new(&data);
        let region_id = world.regions[0].id.clone();

        // Two hunters: a mighty warrior (claims the kill) and a lesser ranger.
        world.heroes.retain(|h| h.region_id != region_id);
        use crate::data::{HeroRole, HeroSeed};
        let mut champion = Hero::from_seed(&HeroSeed {
            id: "champ".to_owned(),
            name: "Bramwell the Bold".to_owned(),
            role: HeroRole::Warrior,
            region_id: region_id.clone(),
            level: 20,
            age: 30,
        });
        champion.renown = 0.0;
        let ranger = Hero::from_seed(&HeroSeed {
            id: "ranger".to_owned(),
            name: "A Lesser Scout".to_owned(),
            role: HeroRole::Ranger,
            region_id: region_id.clone(),
            level: 3,
            age: 30,
        });
        world.heroes.push(champion);
        world.heroes.push(ranger);
        world.monsters.push(Monster {
            id: "m".to_owned(),
            name: "The Doomed Beast".to_owned(),
            type_id: "dire_pack".to_owned(),
            region_id,
            ferocity: 1.5,
            age: 0,
        });

        let felled = run(&mut world, &data, &balance);

        assert!(world.monsters.is_empty(), "the beast should be slain");
        let champ = world.heroes.iter().find(|h| h.id == "champ").unwrap();
        let ranger = world.heroes.iter().find(|h| h.id == "ranger").unwrap();
        assert_eq!(
            champ.renown, balance.slay_renown,
            "the mightiest hunter should earn the renown of the kill"
        );
        assert_eq!(ranger.renown, 0.0, "the lesser hunter earns none");
        // The kill is reported so the caller can commemorate it in myth.
        assert_eq!(
            felled,
            vec![(
                "Bramwell the Bold".to_owned(),
                "The Doomed Beast".to_owned(),
                world.regions[0].id.clone()
            )],
            "the felled beast and its slayer are reported"
        );
    }

    #[test]
    fn an_unopposed_beast_grows_fiercer() {
        let data = GameData::load().unwrap();
        let mut balance = data.balance.monster.clone();
        balance.emergence_chance = 0.0;
        let mut world = WorldState::new(&data);
        let region_id = world.regions[0].id.clone();
        // No hunters anywhere in the region.
        world.heroes.retain(|h| h.region_id != region_id);
        world.monsters.push(Monster {
            id: "m".to_owned(),
            name: "The Growing Terror".to_owned(),
            type_id: "dire_pack".to_owned(),
            region_id,
            ferocity: 1.0,
            age: 0,
        });

        run(&mut world, &data, &balance);
        assert!(
            world.monsters[0].ferocity > 1.0,
            "an unopposed beast should grow fiercer"
        );
    }
}
