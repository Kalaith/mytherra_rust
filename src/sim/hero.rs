//! Per-tick hero lifecycle: level-up, aging, death, and region movement
//! (GDD 5.4). All randomness flows through the world-owned `SeededRng` so the
//! sim stays deterministic and auditable.

use crate::data::strings::ChronicleText;
use crate::data::{fill, HeroBalance, HeroRole, MigrationBalance};
use crate::world::{Chronicle, EventKind, Hero, Region};
use macroquad_toolkit::rng::SeededRng;

/// Advance every living hero by one world tick.
pub fn tick_heroes(
    heroes: &mut [Hero],
    regions: &[Region],
    rng: &mut SeededRng,
    balance: &HeroBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    for hero in heroes.iter_mut() {
        if !hero.is_alive {
            continue;
        }

        if rng.chance(hero.level_up_chance(balance)) {
            hero.level += 1;
            hero.renown += balance.renown.per_level;
            chronicle.push(
                year,
                EventKind::Hero,
                fill(
                    &text.hero_level_up,
                    &[
                        ("hero", hero.name.clone()),
                        ("region", region_name(regions, &hero.region_id)),
                        ("level", hero.level.to_string()),
                    ],
                ),
            );
        }

        hero.age += 1;

        if rolls_death(hero, regions, rng, balance) {
            hero.is_alive = false;
            chronicle.push(
                year,
                EventKind::Hero,
                fill(
                    &text.hero_death,
                    &[
                        ("hero", hero.name.clone()),
                        ("region", region_name(regions, &hero.region_id)),
                    ],
                ),
            );
            continue;
        }

        if rng.chance(balance.move_chance) {
            if let Some(dest) =
                pick_destination(regions, &hero.region_id, hero.role, rng, &balance.migration)
            {
                hero.region_id = dest;
            }
        }
    }
}

/// Death roll for one hero: elders past their life expectancy roll a flat
/// chance; younger heroes face a danger-scaled, level-mitigated chance.
fn rolls_death(
    hero: &Hero,
    regions: &[Region],
    rng: &mut SeededRng,
    balance: &HeroBalance,
) -> bool {
    let death = &balance.death;
    if hero.age as f32 > hero.life_expectancy(balance) {
        return rng.chance(death.elder_roll);
    }
    let danger = region_danger(regions, &hero.region_id);
    rng.chance(danger_death_chance(hero, danger, balance))
}

/// A young hero's per-tick chance of a violent death. Level and hard-won renown
/// both stave it off — a legend clings to life against the odds — but never
/// below the floor.
fn danger_death_chance(hero: &Hero, danger: f32, balance: &HeroBalance) -> f32 {
    let death = &balance.death;
    (danger / death.danger_divisor
        - hero.level as f32 / death.level_divisor
        - hero.renown * balance.renown.survival_coeff)
        .max(death.min_chance)
}

fn region_danger(regions: &[Region], region_id: &str) -> f32 {
    regions
        .iter()
        .find(|r| r.id == region_id)
        .map(|r| r.danger)
        .unwrap_or(0.0)
}

fn region_name(regions: &[Region], region_id: &str) -> String {
    regions
        .iter()
        .find(|r| r.id == region_id)
        .map(|r| r.name.clone())
        .unwrap_or_else(|| region_id.to_owned())
}

/// How strongly a region draws a hero of the given role (GDD 5.4). Each role
/// weights the region's stats differently, floored so the pull is always
/// positive. This is what makes warriors flow toward danger and scholars toward
/// settled, cultured lands.
fn attractiveness(region: &Region, role: HeroRole, mig: &MigrationBalance) -> f32 {
    let w = mig.roles.get(role);
    (mig.base_weight
        + w.prosperity * region.prosperity
        + w.danger * region.danger
        + w.magic * region.magic_affinity
        + w.culture * region.cultural_influence)
        .max(mig.min_weight)
}

/// Pick a destination region other than the hero's current one, weighted by how
/// attractive each is to the hero's role. Deterministic given the RNG state: a
/// single roll walks the cumulative weight.
fn pick_destination(
    regions: &[Region],
    current: &str,
    role: HeroRole,
    rng: &mut SeededRng,
    mig: &MigrationBalance,
) -> Option<String> {
    let candidates: Vec<(&str, f32)> = regions
        .iter()
        .filter(|r| r.id != current)
        .map(|r| (r.id.as_str(), attractiveness(r, role, mig)))
        .collect();
    if candidates.is_empty() {
        return None;
    }
    let total: f32 = candidates.iter().map(|(_, w)| *w).sum();
    let mut roll = rng.next_f32() * total;
    for (id, weight) in &candidates {
        roll -= *weight;
        if roll <= 0.0 {
            return Some((*id).to_owned());
        }
    }
    // Floating-point fallthrough: take the last candidate.
    Some(candidates[candidates.len() - 1].0.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{ClimateType, Culture, GameData, HeroSeed, RegionSeed};
    use crate::world::WorldState;

    fn region(id: &str, prosperity: f32, danger: f32, magic: f32, culture: f32) -> Region {
        let balance = GameData::load().unwrap().balance.region;
        Region::from_seed(
            &RegionSeed {
                id: id.to_owned(),
                name: id.to_owned(),
                climate: ClimateType::Temperate,
                culture: Culture::Martial,
                prosperity,
                chaos: 30.0,
                danger,
                magic_affinity: magic,
                population: 5000.0,
                cultural_influence: culture,
                divine_resonance: 50.0,
            },
            &balance,
        )
    }

    fn hero(id: &str, role: HeroRole, region_id: &str) -> Hero {
        Hero::from_seed(&HeroSeed {
            id: id.to_owned(),
            name: id.to_owned(),
            role,
            region_id: region_id.to_owned(),
            level: 5,
            age: 30,
        })
    }

    #[test]
    fn migration_weights_pull_each_role_differently() {
        let data = GameData::load().unwrap();
        let mig = &data.balance.hero.migration;
        let dangerous = region("war", 25.0, 90.0, 20.0, 20.0);
        let settled = region("haven", 90.0, 10.0, 30.0, 85.0);

        // A warrior is drawn to conflict; a scholar toward settled, cultured land.
        assert!(
            attractiveness(&dangerous, HeroRole::Warrior, mig)
                > attractiveness(&settled, HeroRole::Warrior, mig)
        );
        assert!(
            attractiveness(&settled, HeroRole::Scholar, mig)
                > attractiveness(&dangerous, HeroRole::Scholar, mig)
        );
        // A mage follows magic.
        let arcane = region("spire", 50.0, 30.0, 95.0, 40.0);
        assert!(
            attractiveness(&arcane, HeroRole::Mage, mig)
                > attractiveness(&settled, HeroRole::Mage, mig)
        );
    }

    #[test]
    fn warriors_gather_where_scholars_flee() {
        let data = GameData::load().unwrap();
        let mut balance = data.balance.hero.clone();
        // Sample steady-state migration, not the death/aging system: let heroes
        // move often and live indefinitely so the distribution is what's tested.
        balance.move_chance = 0.5;
        balance.death.min_chance = 0.0;
        balance.death.elder_roll = 0.0;
        balance.death.danger_divisor = 1.0e9; // war would otherwise thin the warriors
        balance.life_expectancy_base = 1.0e6;
        let mut world = WorldState::new(&data);
        // Three regions so the weighted choice actually has alternatives.
        world.regions = vec![
            region("war", 30.0, 70.0, 20.0, 20.0),
            region("haven", 85.0, 10.0, 30.0, 85.0),
            region("wild", 45.0, 45.0, 40.0, 30.0),
        ];
        // Everyone starts in the neutral middle; roles should sort themselves out.
        world.heroes = (0..12)
            .map(|i| {
                let role = if i % 2 == 0 {
                    HeroRole::Warrior
                } else {
                    HeroRole::Scholar
                };
                hero(&format!("h{i}"), role, "wild")
            })
            .collect();

        for _ in 0..150 {
            tick_heroes(
                &mut world.heroes,
                &world.regions,
                &mut world.rng,
                &balance,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
        }

        let warriors_in_war = world
            .heroes
            .iter()
            .filter(|h| h.is_alive && h.role == HeroRole::Warrior && h.region_id == "war")
            .count();
        let scholars_in_war = world
            .heroes
            .iter()
            .filter(|h| h.is_alive && h.role == HeroRole::Scholar && h.region_id == "war")
            .count();
        assert!(
            warriors_in_war > scholars_in_war,
            "warriors ({warriors_in_war}) should out-gather scholars ({scholars_in_war}) in the war region"
        );
    }

    #[test]
    fn renown_lowers_a_heros_danger_death() {
        let data = GameData::load().unwrap();
        let world = WorldState::new(&data);
        let mut famed = world.heroes[0].clone();
        famed.renown = 200.0;
        let mut unknown = famed.clone();
        unknown.renown = 0.0;
        assert!(
            danger_death_chance(&famed, 80.0, &data.balance.hero)
                < danger_death_chance(&unknown, 80.0, &data.balance.hero),
            "a renowned hero should be harder for danger to kill"
        );
    }

    #[test]
    fn renown_accrues_as_heroes_level() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        for _ in 0..100 {
            tick_heroes(
                &mut world.heroes,
                &world.regions,
                &mut world.rng,
                &data.balance.hero,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
        }
        assert!(
            world.heroes.iter().any(|h| h.renown > 0.0),
            "some hero should have earned renown by levelling"
        );
    }

    #[test]
    fn heroes_age_each_tick() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let before: Vec<u32> = world.heroes.iter().map(|h| h.age).collect();
        tick_heroes(
            &mut world.heroes,
            &world.regions,
            &mut world.rng,
            &data.balance.hero,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        for (hero, before_age) in world.heroes.iter().zip(before) {
            if hero.is_alive {
                assert_eq!(hero.age, before_age + 1);
            }
        }
    }

    #[test]
    fn simulation_is_deterministic_for_a_seed() {
        let data = GameData::load().unwrap();
        let run = || {
            let mut world = WorldState::new(&data);
            for _ in 0..50 {
                tick_heroes(
                    &mut world.heroes,
                    &world.regions,
                    &mut world.rng,
                    &data.balance.hero,
                    &mut world.chronicle,
                    &data.strings.chronicle,
                    world.year,
                );
            }
            world
                .heroes
                .iter()
                .map(|h| (h.level, h.age, h.is_alive, h.region_id.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }
}
