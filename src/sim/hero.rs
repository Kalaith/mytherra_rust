//! Per-tick hero lifecycle: level-up, aging, death, and region movement
//! (GDD 5.4). All randomness flows through the world-owned `SeededRng` so the
//! sim stays deterministic and auditable.

use crate::data::strings::ChronicleText;
use crate::data::{fill, HeroBalance};
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
            if let Some(dest) = pick_other_region(regions, &hero.region_id, rng) {
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
    let chance = (danger / death.danger_divisor - hero.level as f32 / death.level_divisor)
        .max(death.min_chance);
    rng.chance(chance)
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

/// Pick a random region id other than the hero's current one.
fn pick_other_region(regions: &[Region], current: &str, rng: &mut SeededRng) -> Option<String> {
    let candidates: Vec<&str> = regions
        .iter()
        .map(|r| r.id.as_str())
        .filter(|id| *id != current)
        .collect();
    if candidates.is_empty() {
        None
    } else {
        Some(candidates[rng.below(candidates.len())].to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::world::WorldState;

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
