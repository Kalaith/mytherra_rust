//! The era system (GDD 5.7): each tick recomputes era pressure from five
//! weighted triggers, and when the era's calendar length elapses or pressure
//! breaks the threshold, a transition reshapes the world — reincarnating or
//! killing heroes, spawning descendants, expiring boundary-spanning bets, and
//! renewing the land. Randomness flows through the world RNG.

use crate::data::{fill, GameData, HeroRole};
use crate::world::{
    compute_scores, generate_era_name, EraRecord, EventKind, Hero, PlayerState, WorldState,
};

/// Recompute era pressure and transition if due.
pub fn tick_era(world: &mut WorldState, player: &mut PlayerState, data: &GameData) {
    let balance = &data.balance.era;
    let pending_stake: i64 = player
        .bets
        .iter()
        .filter(|b| b.resolved.is_none())
        .map(|b| b.stake)
        .sum();
    let scores = compute_scores(
        &world.regions,
        &world.heroes,
        &world.magic_paths,
        player.favor,
        data.config.max_favor,
        pending_stake,
        world.conquest_momentum,
        balance,
    );
    let (dominant, pressure) = scores.dominant();
    world.era.pressure = pressure;
    world.era.dominant_trigger = dominant;

    // Conquests fade from living memory: bleed the momentum they left behind.
    world.conquest_momentum = (world.conquest_momentum - balance.conquest_momentum_decay).max(0.0);

    let elapsed = world.year.saturating_sub(world.era.start_year);
    if elapsed >= balance.era_length || pressure >= balance.breaking_threshold {
        transition(world, player, data);
    }
}

fn transition(world: &mut WorldState, player: &mut PlayerState, data: &GameData) {
    let balance = &data.balance.era;

    world.era_history.push(EraRecord {
        number: world.era.number,
        name: world.era.name.clone(),
        start_year: world.era.start_year,
        end_year: world.year,
        trigger: world.era.dominant_trigger,
        pressure: world.era.pressure,
    });
    if world.era_history.len() > 20 {
        world.era_history.remove(0);
    }

    // Heroes reincarnate (age reset, scaled level) or die.
    for hero in world.heroes.iter_mut() {
        if !hero.is_alive {
            continue;
        }
        let dies = hero.age >= balance.death_age || world.rng.chance(balance.death_chance);
        if dies {
            hero.is_alive = false;
        } else {
            hero.age = reincarnate_age(
                &mut world.rng,
                balance.reincarnate_age_min,
                balance.reincarnate_age_max,
            );
            hero.level = ((hero.level as f32 * balance.hero_level_scale) as u32).max(1);
        }
    }

    // Champions of the departed pass with them.
    player
        .champions
        .retain(|c| world.heroes.iter().any(|h| h.id == c.hero_id && h.is_alive));

    // Descendant heroes rise.
    let region_ids: Vec<String> = world.regions.iter().map(|r| r.id.clone()).collect();
    let span = (balance.descendant_max - balance.descendant_min + 1).max(1) as usize;
    let count = balance.descendant_min + world.rng.below(span) as u32;
    for _ in 0..count {
        world.hero_seq += 1;
        let region_id = region_ids
            .get(world.rng.below(region_ids.len().max(1)))
            .cloned()
            .unwrap_or_default();
        let prefix = world
            .rng
            .choose(&data.era_names.prefixes)
            .cloned()
            .unwrap_or_default();
        let title = world
            .rng
            .choose(&data.era_names.descendant_titles)
            .cloned()
            .unwrap_or_default();
        let role = match world.rng.below(4) {
            0 => HeroRole::Warrior,
            1 => HeroRole::Mage,
            2 => HeroRole::Scholar,
            _ => HeroRole::Ranger,
        };
        world.heroes.push(Hero {
            id: format!("descendant-{}", world.hero_seq),
            name: format!("{prefix} {title}"),
            role,
            region_id,
            level: 1,
            age: reincarnate_age(
                &mut world.rng,
                balance.reincarnate_age_min,
                balance.reincarnate_age_max,
            ),
            is_alive: true,
        });
    }

    // Bets spanning the boundary are force-expired.
    for bet in player.bets.iter_mut() {
        if bet.resolved.is_none() {
            bet.resolved = Some(false);
        }
    }

    // The land is renewed — plus the mark the ending age's trigger leaves, so a
    // Collapse rebuilds prosperity while a Cataclysm leaves the new world scarred.
    let aftermath = balance.aftermath.get(world.era.dominant_trigger);
    for region in world.regions.iter_mut() {
        region.apply_deltas(
            balance.renewal_prosperity + aftermath.prosperity,
            balance.renewal_chaos + aftermath.chaos,
            balance.renewal_danger + aftermath.danger,
            aftermath.magic,
            &data.balance.region,
        );
    }
    world.weather.clear();

    // A new era dawns.
    world.era.number += 1;
    world.era.name = generate_era_name(&data.era_names, &mut world.rng);
    world.era.start_year = world.year;
    world.era.pressure = 0.0;

    let trigger = world
        .era_history
        .last()
        .map(|r| r.trigger.label())
        .unwrap_or("Cataclysm");
    world.chronicle.push(
        world.year,
        EventKind::System,
        fill(
            &data.strings.chronicle.era_transition,
            &[
                ("era", world.era.name.clone()),
                ("trigger", trigger.to_owned()),
            ],
        ),
    );
}

fn reincarnate_age(rng: &mut macroquad_toolkit::rng::SeededRng, min: u32, max: u32) -> u32 {
    let span = (max - min + 1).max(1) as usize;
    min + rng.below(span) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldState;

    #[test]
    fn breaking_pressure_forces_a_transition() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut player = PlayerState::new(&data.config);
        // Drive every region to maximal danger/chaos so pressure breaks.
        for region in &mut world.regions {
            region.danger = 100.0;
            region.chaos = 100.0;
            region.prosperity = 0.0;
            region.refresh_status(&data.balance.region);
        }
        let era_before = world.era.number;
        tick_era(&mut world, &mut player, &data);
        assert!(world.era.number > era_before);
        assert_eq!(world.era_history.len(), 1);
    }

    #[test]
    fn conquest_momentum_raises_conquest_pressure_and_decays() {
        use crate::world::compute_scores;
        let data = GameData::load().unwrap();
        let balance = &data.balance.era;
        let mut world = WorldState::new(&data);

        // Same world, scored with and without recent conquests.
        let quiet = compute_scores(
            &world.regions,
            &world.heroes,
            &world.magic_paths,
            100,
            data.config.max_favor,
            0,
            0.0,
            balance,
        );
        let warlike = compute_scores(
            &world.regions,
            &world.heroes,
            &world.magic_paths,
            100,
            data.config.max_favor,
            0,
            50.0,
            balance,
        );
        assert!(
            warlike.conquest > quiet.conquest,
            "recent conquests should raise Conquest pressure"
        );
        assert!(
            (warlike.conquest - quiet.conquest - 50.0 * balance.conquest_momentum_weight).abs()
                < 0.01,
            "the momentum term should be exactly weight * momentum"
        );

        // And the momentum bleeds off over ticks.
        world.conquest_momentum = 40.0;
        let mut player = PlayerState::new(&data.config);
        tick_era(&mut world, &mut player, &data);
        assert!(world.conquest_momentum < 40.0);
    }

    #[test]
    fn calm_world_stays_in_its_era() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut player = PlayerState::new(&data.config);
        tick_era(&mut world, &mut player, &data);
        assert_eq!(world.era.number, 1);
    }

    #[test]
    fn aftermath_reflects_each_trigger_theme() {
        use crate::data::EraTrigger;
        let a = GameData::load().unwrap().balance.era.aftermath;
        assert!(
            a.get(EraTrigger::Collapse).prosperity > 0.0,
            "a Collapse should rebuild prosperity"
        );
        assert!(
            a.get(EraTrigger::Conquest).danger > 0.0,
            "a Conquest should leave lingering danger"
        );
        assert!(
            a.get(EraTrigger::MagicalRupture).magic > 0.0,
            "a Rupture should leave arcane residue"
        );
        assert!(
            a.get(EraTrigger::DivineWar).chaos > 0.0,
            "a Divine War should leave chaos"
        );
        assert!(
            a.get(EraTrigger::Cataclysm).danger > 0.0,
            "a Cataclysm should scar the new world"
        );
    }
}
