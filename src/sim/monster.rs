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
        monsters, regions, heroes, types, seq, balance, rng, chronicle, text, year,
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
        // The hunt: resident hunters grind the beast down — but who can fight it
        // depends on its nature. Steel bites a natural predator full, but only
        // weakly bites an arcane horror, which a Mage must answer in kind. Left
        // wholly unopposed, the beast grows into a greater terror.
        let might = hunter_might(heroes, &monster.region_id, ty.arcane, balance);
        if might > 0.0 {
            monster.ferocity -= might * balance.slay_per_might;
        } else {
            monster.ferocity += balance.ferocity_growth;
        }
    }

    // Beasts worn below the floor are slain (or, where no hunter remains, driven
    // off); the mightiest resident hunter claims the kill and its renown.
    let slain: Vec<(String, String, bool)> = monsters
        .iter()
        .filter(|m| m.ferocity < balance.min_ferocity)
        .map(|m| {
            let arcane = types
                .iter()
                .find(|t| t.id == m.type_id)
                .is_some_and(|t| t.arcane);
            (m.region_id.clone(), m.name.clone(), arcane)
        })
        .collect();
    monsters.retain(|m| m.ferocity >= balance.min_ferocity);

    let mut felled: Vec<BeastSlain> = Vec::new();
    for (region_id, name, arcane) in slain {
        let slayer = heroes
            .iter_mut()
            .filter(|h| h.is_alive && h.region_id == region_id && hunts(h.role, arcane))
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
    heroes: &[Hero],
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
        // Peril breeds beasts, but resident Rangers ward the wilds against them:
        // the more dangerous the land the likelier one stalks forth, the more its
        // rangers patrol the fewer do (GDD 5.2 <-> 5.4).
        let ward = ranger_ward(heroes, &region.id) * balance.ranger_ward;
        let chance = (balance.emergence_chance + region.danger * balance.emergence_danger_coeff
            - ward)
            .max(0.0);
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

/// Whether a hero of this role can meaningfully hunt a beast of this nature:
/// Warriors and Rangers face any predator, while a Mage joins the hunt only
/// against an arcane horror — magic answered in kind (GDD 5.2 <-> 5.4).
fn hunts(role: HeroRole, arcane: bool) -> bool {
    match role {
        HeroRole::Warrior | HeroRole::Ranger => true,
        HeroRole::Mage => arcane,
        _ => false,
    }
}

/// The combined levels of a region's living Rangers — the strength of its patrol
/// warding the wilds against beasts before they emerge.
fn ranger_ward(heroes: &[Hero], region_id: &str) -> f32 {
    heroes
        .iter()
        .filter(|h| h.is_alive && h.role == HeroRole::Ranger && h.region_id == region_id)
        .map(|h| h.level as f32)
        .sum()
}

/// The might a region can bring to bear against a beast, summed over its living
/// hunters and weighted by how well each answers the beast's nature: steel bites
/// a natural predator in full but an arcane horror only weakly, while a Mage is
/// the surest bane of the arcane and no help at all against a mundane beast.
fn hunter_might(heroes: &[Hero], region_id: &str, arcane: bool, balance: &MonsterBalance) -> f32 {
    heroes
        .iter()
        .filter(|h| h.is_alive && h.region_id == region_id)
        .map(|h| {
            let effectiveness = match h.role {
                HeroRole::Warrior | HeroRole::Ranger => {
                    if arcane {
                        balance.arcane_martial_effectiveness
                    } else {
                        1.0
                    }
                }
                HeroRole::Mage if arcane => balance.mage_arcane_effectiveness,
                _ => 0.0,
            };
            h.level as f32 * effectiveness
        })
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
    fn resident_rangers_ward_the_wilds_against_beasts() {
        // The same perilous region breeds fewer beasts when Rangers patrol it than
        // when it is left unwarded (GDD 5.2 <-> 5.4).
        use crate::data::{HeroRole, HeroSeed};
        let data = GameData::load().unwrap();
        let emergences = |rangers: usize| {
            let mut world = WorldState::new(&data);
            world.regions.truncate(1);
            world.regions[0].danger = 95.0;
            world.regions[0].magic_affinity = 0.0;
            let region_id = world.regions[0].id.clone();
            world.heroes.retain(|h| h.region_id != region_id);
            for i in 0..rangers {
                world.heroes.push(Hero::from_seed(&HeroSeed {
                    id: format!("r{i}"),
                    name: format!("Ranger {i}"),
                    role: HeroRole::Ranger,
                    region_id: region_id.clone(),
                    level: 20,
                    age: 30,
                }));
            }
            let mut count = 0;
            for _ in 0..600 {
                world.monsters.clear(); // isolate emergence odds
                run(&mut world, &data, &data.balance.monster);
                count += world.monsters.len();
            }
            count
        };
        assert!(
            emergences(0) > emergences(4),
            "a ranger-warded land should breed fewer beasts than an unwarded one"
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
    fn an_arcane_horror_resists_steel_but_falls_to_a_mage() {
        // A Warrior wears an arcane beast down only weakly; a Mage of the same
        // level answers it in kind and cuts far deeper (GDD 5.2 <-> 5.4).
        use crate::data::{HeroRole, HeroSeed};
        let data = GameData::load().unwrap();
        let balance = &data.balance.monster;

        // Ferocity lost in one tick to a single level-10 hunter of the given role,
        // set against an arcane Shadow Wyrm.
        let bite = |role: HeroRole| {
            let mut world = WorldState::new(&data);
            let mut b = balance.clone();
            b.emergence_chance = 0.0;
            let region_id = world.regions[0].id.clone();
            world.heroes.retain(|h| h.region_id != region_id);
            world.heroes.push(Hero::from_seed(&HeroSeed {
                id: "h".to_owned(),
                name: "H".to_owned(),
                role,
                region_id: region_id.clone(),
                level: 10,
                age: 30,
            }));
            world.monsters.push(Monster {
                id: "m".to_owned(),
                name: "The Wyrm".to_owned(),
                type_id: "shadow_wyrm".to_owned(), // arcane
                region_id,
                ferocity: 5.0,
                age: 0,
            });
            run(&mut world, &data, &b);
            5.0 - world.monsters.first().map(|m| m.ferocity).unwrap_or(0.0)
        };

        let warrior_bite = bite(HeroRole::Warrior);
        let mage_bite = bite(HeroRole::Mage);
        assert!(
            warrior_bite > 0.0,
            "steel should still bite an arcane beast, if weakly"
        );
        assert!(
            mage_bite > warrior_bite,
            "a Mage should cut deeper into an arcane horror than a Warrior ({mage_bite} vs {warrior_bite})"
        );
    }

    #[test]
    fn a_mage_is_no_help_against_a_natural_predator() {
        // Against a mundane beast a lone Mage lends nothing, so the pack grows
        // unchecked as if unopposed (GDD 5.2).
        use crate::data::{HeroRole, HeroSeed};
        let data = GameData::load().unwrap();
        let mut balance = data.balance.monster.clone();
        balance.emergence_chance = 0.0;
        let mut world = WorldState::new(&data);
        let region_id = world.regions[0].id.clone();
        world.heroes.retain(|h| h.region_id != region_id);
        world.heroes.push(Hero::from_seed(&HeroSeed {
            id: "mage".to_owned(),
            name: "A Mage".to_owned(),
            role: HeroRole::Mage,
            region_id: region_id.clone(),
            level: 20,
            age: 30,
        }));
        world.monsters.push(Monster {
            id: "m".to_owned(),
            name: "The Pack".to_owned(),
            type_id: "dire_pack".to_owned(), // natural
            region_id,
            ferocity: 1.0,
            age: 0,
        });

        run(&mut world, &data, &balance);
        assert!(
            world.monsters[0].ferocity > 1.0,
            "a mage can't hunt a natural pack, so it grows unopposed"
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
