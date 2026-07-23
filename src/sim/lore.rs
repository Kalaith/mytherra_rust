//! Per-tick lore (GDD 5.6 <-> 5.3): the accumulated practical knowledge of a
//! civilization — medicine, agriculture, engineering, governance — as distinct
//! from the arcane power of magic and the prominence of culture. Each region's
//! lore drifts, slowly, toward a target set by the scholars who dwell there, the
//! great libraries that stand there, the magic the world has brought to Known, and
//! the wealth that affords the leisure to study. A learned land is not richer or
//! mightier for its knowledge, but it is more resilient: it tends its sick through
//! a plague and stores its grain against a dearth where an ignorant land only
//! buries its dead. Deterministic: the drift reads world state, no RNG.

use crate::data::{HeroRole, LoreBalance};
use crate::world::{Hero, Landmark, MagicPath, MagicState, Region};

pub fn tick_lore(
    regions: &mut [Region],
    heroes: &[Hero],
    landmarks: &[Landmark],
    magic_paths: &[MagicPath],
    balance: &LoreBalance,
) {
    // The mastery of the arcane lifts the whole world's understanding, so every
    // Known magic path raises the lore *ceiling* every land drifts toward — read
    // once, it applies to all.
    let known = magic_paths
        .iter()
        .filter(|p| p.state == MagicState::Known)
        .count() as f32;
    let world_learning = known * balance.per_known_path;

    for region in regions.iter_mut() {
        // The learned who dwell here — scholars and mages both study the mundane as
        // the arcane — are the wellspring of the land's knowledge.
        let scholars = heroes
            .iter()
            .filter(|h| {
                h.is_alive
                    && matches!(h.role, HeroRole::Scholar | HeroRole::Mage)
                    && h.region_id == region.id
            })
            .count() as f32;
        // Its great libraries and colleges — the scholarly and mystical wonders —
        // are the storehouses of its learning, the older and prouder the more so.
        let libraries: f32 = landmarks
            .iter()
            .filter(|l| {
                l.region_id == region.id
                    && matches!(
                        l.culture,
                        crate::data::Culture::Scholarly | crate::data::Culture::Mystical
                    )
            })
            .map(|l| l.influence * l.stature)
            .sum();
        // And the wealth to afford the leisure of study, above the neutral line.
        let affluence = (region.prosperity - 50.0).max(0.0) * balance.prosperity_coeff;

        let target = (balance.base
            + scholars * balance.per_scholar
            + libraries * balance.per_learned_landmark
            + world_learning
            + affluence)
            .clamp(0.0, 100.0);

        // Knowledge is slow to gather and slow to lose: lore creeps toward its
        // target rather than leaping to it.
        region.add_lore((target - region.lore) * balance.drift_rate);
    }
}

/// The fraction of a demographic toll a region's lore averts, 0..=`relief` (GDD
/// 5.6 <-> 5.3): a learned land loses fewer of its people to a disaster, scaled
/// linearly from a wholly ignorant land (nothing) to a fully-learned one
/// (`relief`). Shared by the plague and famine tolls so knowledge softens both by
/// one rule.
pub fn toll_relief(region: &Region, relief: f32) -> f32 {
    (region.lore / 100.0).clamp(0.0, 1.0) * relief
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{GameData, HeroSeed, LandmarkSeed};
    use crate::world::WorldState;

    fn run(world: &mut WorldState, data: &GameData) {
        tick_lore(
            &mut world.regions,
            &world.heroes,
            &world.landmarks,
            &world.magic_paths,
            &data.balance.lore,
        );
    }

    #[test]
    fn a_land_of_scholars_grows_learned_and_a_barren_one_does_not() {
        let data = GameData::load().unwrap();
        let lore_after = |scholars: usize| {
            let mut world = WorldState::new(&data);
            world.regions.truncate(1);
            world.landmarks.clear();
            world.magic_paths.clear();
            let region_id = world.regions[0].id.clone();
            world.regions[0].prosperity = 50.0; // neutral, so only scholars matter
            world.regions[0].lore = 20.0;
            world.heroes = (0..scholars)
                .map(|i| {
                    Hero::from_seed(&HeroSeed {
                        id: format!("s{i}"),
                        name: format!("Scholar {i}"),
                        role: HeroRole::Scholar,
                        region_id: region_id.clone(),
                        level: 5,
                        age: 30,
                    })
                })
                .collect();
            for _ in 0..300 {
                run(&mut world, &data);
            }
            world.regions[0].lore
        };
        assert!(
            lore_after(4) > lore_after(0),
            "a land of scholars should grow far more learned than a barren one"
        );
    }

    #[test]
    fn a_great_library_stores_a_lands_learning() {
        let data = GameData::load().unwrap();
        let lore_after = |with_library: bool| {
            let mut world = WorldState::new(&data);
            world.regions.truncate(1);
            world.heroes.clear();
            world.magic_paths.clear();
            let region_id = world.regions[0].id.clone();
            world.regions[0].prosperity = 50.0;
            world.regions[0].lore = 20.0;
            world.landmarks.clear();
            if with_library {
                let mut l = Landmark::from_seed(&LandmarkSeed {
                    id: "lib".to_owned(),
                    name: "The Great Library".to_owned(),
                    region_id: region_id.clone(),
                    culture: crate::data::Culture::Scholarly,
                    influence: 5.0,
                });
                l.stature = 40.0;
                world.landmarks.push(l);
            }
            for _ in 0..300 {
                run(&mut world, &data);
            }
            world.regions[0].lore
        };
        assert!(
            lore_after(true) > lore_after(false),
            "a land with a great library should grow more learned than one without"
        );
    }

    #[test]
    fn lore_reliefs_scale_from_ignorance_to_mastery() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let relief = 0.5;
        world.regions[0].lore = 0.0;
        assert_eq!(toll_relief(&world.regions[0], relief), 0.0);
        world.regions[0].lore = 100.0;
        assert_eq!(toll_relief(&world.regions[0], relief), relief);
        world.regions[0].lore = 50.0;
        assert!((toll_relief(&world.regions[0], relief) - relief * 0.5).abs() < 1e-4);
    }
}
