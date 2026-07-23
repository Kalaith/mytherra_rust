//! The era system (GDD 5.7): each tick recomputes era pressure from five
//! weighted triggers, and when the era's calendar length elapses or pressure
//! breaks the threshold, a transition reshapes the world — reincarnating or
//! killing heroes, spawning descendants, expiring boundary-spanning bets, and
//! renewing the land. Randomness flows through the world RNG.

use crate::data::{fill, Culture, GameData, HeroRole};
use crate::sim::culture::culture_role;
#[cfg(test)]
use crate::world::PlayerState;
use crate::world::{
    compute_scores, generate_era_name, Bet, EraRecord, EventKind, Hero, WorldState,
};

/// Recompute era pressure and transition if due, for a single deity — the
/// one-player composition of [`tick_era_world`] + [`expire_boundary_bets`] that
/// the era tests drive in isolation. Production advances the era through the two
/// halves directly (see `sim::tick_shared`).
#[cfg(test)]
pub fn tick_era(world: &mut WorldState, player: &mut PlayerState, data: &GameData) {
    // Aggregates over one player equal the player itself, so this is
    // byte-identical to the pre-multiplayer era tick.
    let pending_stake: i64 = player
        .bets
        .iter()
        .filter(|b| b.resolved.is_none())
        .map(|b| b.stake)
        .sum();
    if tick_era_world(
        world,
        player.favor,
        data.config.max_favor,
        pending_stake,
        data,
    ) {
        expire_boundary_bets(&mut player.bets, &mut player.favor);
    }
}

/// Recompute era pressure from the shared world plus the deities' *aggregate*
/// favor and pending stake, and transition the world if due. Player state is
/// untouched here — a transition's per-deity effect (boundary bets) is
/// [`expire_boundary_bets`], applied to each connected player afterward.
/// Returns whether an age turned this tick.
pub(crate) fn tick_era_world(
    world: &mut WorldState,
    total_favor: i64,
    total_max_favor: i64,
    total_pending_stake: i64,
    data: &GameData,
) -> bool {
    let balance = &data.balance.era;
    let wrath = crate::world::pantheon_wrath(&world.pantheon, data.balance.pantheon.drift_target);
    // What share of the world lies under plague — one plague per region, so the
    // afflicted count is the distinct region count (GDD 5.7 <-> 5.3).
    let mut afflicted: Vec<&str> = world.plagues.iter().map(|p| p.region_id.as_str()).collect();
    afflicted.sort_unstable();
    afflicted.dedup();
    let plague_ratio = afflicted.len() as f32 / world.regions.len().max(1) as f32;
    // What share of the world's granaries have failed — famine is a per-region
    // flag, so this is the fraction of regions gripped by dearth (GDD 5.7 <-> 5.3).
    let famine_ratio = world.regions.iter().filter(|r| r.famine).count() as f32
        / world.regions.len().max(1) as f32;
    let scores = compute_scores(
        &world.regions,
        &world.heroes,
        &world.magic_paths,
        total_favor,
        total_max_favor,
        total_pending_stake,
        world.conquest_momentum,
        world.secession_momentum,
        plague_ratio,
        famine_ratio,
        wrath,
        balance,
    );
    let (dominant, pressure) = scores.dominant();
    world.era.pressure = pressure;
    world.era.dominant_trigger = dominant;

    // Upheavals fade from living memory: bleed the momentum they left behind.
    world.conquest_momentum = (world.conquest_momentum - balance.conquest_momentum_decay).max(0.0);
    world.secession_momentum =
        (world.secession_momentum - balance.collapse_momentum_decay).max(0.0);

    let elapsed = world.year.saturating_sub(world.era.start_year);
    if elapsed >= balance.era_length || pressure >= balance.breaking_threshold {
        transition(world, data);
        true
    } else {
        false
    }
}

/// A turning age settles a deity's open wagers: a bet that the age would end
/// wins (this transition is exactly its condition), every other pending bet is
/// force-expired (GDD 5.7 <-> 5.5). Player state only — no world, no RNG — so it
/// runs per connected deity after the shared transition.
pub(crate) fn expire_boundary_bets(bets: &mut [Bet], favor: &mut i64) {
    for bet in bets.iter_mut() {
        if bet.resolved.is_none() {
            if bet.predicate == crate::data::BetPredicate::AgeEnds {
                bet.resolved = Some(true);
                *favor += bet.potential_payout;
            } else {
                bet.resolved = Some(false);
            }
        }
    }
}

fn transition(world: &mut WorldState, data: &GameData) {
    let balance = &data.balance.era;
    // How the age ends shapes its transition: a violent trigger is deadlier to
    // heroes and rouses a different number of heirs (GDD 5.7).
    let aftermath = balance.aftermath.get(world.era.dominant_trigger);

    // Heroes reincarnate (age reset, scaled level) or die. Tally the fallen so
    // the closing age's record remembers what its ending cost (GDD 5.7). A
    // legend among the fallen is remembered by name, not just in the count —
    // the closing bookend to its "passes into legend" rise (GDD 5.4 <-> 5.7).
    let legend_bar = data
        .balance
        .hero
        .renown
        .thresholds
        .last()
        .copied()
        .unwrap_or(f32::INFINITY);
    let mut heroes_lost = 0u32;
    let mut fallen_legends: Vec<(String, String)> = Vec::new();
    for hero in world.heroes.iter_mut() {
        if !hero.is_alive {
            continue;
        }
        let death_chance = (balance.death_chance * aftermath.death_mult).clamp(0.0, 1.0);
        let dies = hero.age >= balance.death_age || world.rng.chance(death_chance);
        if dies {
            hero.is_alive = false;
            heroes_lost += 1;
            if hero.renown >= legend_bar {
                fallen_legends.push((hero.name.clone(), hero.region_id.clone()));
            }
        } else {
            hero.age = reincarnate_age(
                &mut world.rng,
                balance.reincarnate_age_min,
                balance.reincarnate_age_max,
            );
            hero.level = ((hero.level as f32 * balance.hero_level_scale) as u32).max(1);
            // Surviving an age is the stuff of legend.
            hero.renown += data.balance.hero.renown.per_era;
        }
    }

    // Even amid an age's collapse, the fall of a legend is its own moment.
    for (name, region_id) in fallen_legends {
        let region = world
            .regions
            .iter()
            .find(|r| r.id == region_id)
            .map(|r| r.name.clone())
            .unwrap_or_default();
        world.chronicle.push(
            world.year,
            EventKind::Hero,
            fill(
                &data.strings.chronicle.hero_legend_death,
                &[("hero", name), ("region", region)],
            ),
        );
    }

    // Champions of the departed are retired (with a chronicled farewell) by
    // `tick_champions` on the next tick — a single retirement path, not a silent
    // cull here.

    // Descendant heroes rise.
    // Region id + culture, so an heir's role can echo the land that bore them.
    let regions_info: Vec<(String, Culture)> = world
        .regions
        .iter()
        .map(|r| (r.id.clone(), r.culture))
        .collect();
    let span = (balance.descendant_max - balance.descendant_min + 1).max(1) as usize;
    let rolled = balance.descendant_min + world.rng.below(span) as u32;
    let count = ((rolled as f32 * aftermath.descendant_mult).round() as u32).max(1);
    for _ in 0..count {
        world.hero_seq += 1;
        let id = format!("descendant-{}", world.hero_seq);
        let (region_id, culture) = regions_info
            .get(world.rng.below(regions_info.len().max(1)))
            .cloned()
            .unwrap_or((String::new(), Culture::Pastoral));
        // A land breeds heirs in its own image more often than not.
        let role = if world.rng.chance(balance.cultural_heir_chance) {
            culture_role(culture)
        } else {
            HeroRole::ALL[world.rng.below(HeroRole::ALL.len())]
        };
        // A descendant born on a house's ancestral seat is its heir, carrying a
        // share of its prestige into the world as renown (GDD 5.4 <-> 5.7). The
        // claim is deterministic, so it never perturbs the transition's rolls.
        let renown = super::house::maybe_inherit(
            &mut world.houses,
            &id,
            &region_id,
            &data.balance.house,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        let age = reincarnate_age(
            &mut world.rng,
            balance.reincarnate_age_min,
            balance.reincarnate_age_max,
        );
        world.heroes.push(Hero {
            id,
            name: descendant_name(&data.hero_names, &mut world.rng),
            role,
            region_id,
            level: 1,
            age,
            is_alive: true,
            renown,
        });
    }

    // The closing age is sealed into the chronicle, now that its toll is known.
    world.era_history.push(EraRecord {
        number: world.era.number,
        name: world.era.name.clone(),
        start_year: world.era.start_year,
        end_year: world.year,
        trigger: world.era.dominant_trigger,
        pressure: world.era.pressure,
        heroes_lost,
        heroes_risen: count,
        // Filled in below, once the razing has run.
        wonders_razed: 0,
    });
    if world.era_history.len() > 20 {
        world.era_history.remove(0);
    }

    // The land is renewed — plus the mark the ending age's trigger leaves, so a
    // Collapse rebuilds prosperity while a Cataclysm leaves the new world scarred.
    for region in world.regions.iter_mut() {
        region.apply_deltas(
            balance.renewal_prosperity + aftermath.prosperity,
            balance.renewal_chaos + aftermath.chaos,
            balance.renewal_danger + aftermath.danger,
            aftermath.magic,
            &data.balance.region,
        );
    }

    // The toll falls on the towns as well as the heroes (GDD 5.7): the age's end
    // claims a share of every settlement's souls. A town gutted below the
    // abandonment floor empties out entirely on the next tick.
    let toll = aftermath.settlement_toll.clamp(0.0, 1.0);
    if toll > 0.0 {
        for settlement in world.settlements.iter_mut() {
            settlement.population = (settlement.population * (1.0 - toll)).max(0.0);
        }
    }

    // A violent age can throw down the old world's wonders (GDD 5.7 <-> 5.2), the
    // counterpart to their founding. Roll per landmark against the world RNG
    // first, then remove the doomed — so the razing is deterministic and the
    // retain touches only locals.
    let raze = aftermath.landmark_raze_chance.clamp(0.0, 1.0);
    if raze > 0.0 && !world.landmarks.is_empty() {
        let doomed: Vec<bool> = (0..world.landmarks.len())
            .map(|_| world.rng.chance(raze))
            .collect();
        let mut fallen: Vec<String> = Vec::new();
        let mut i = 0usize;
        world.landmarks.retain(|l| {
            let keep = !doomed[i];
            if !keep {
                fallen.push(l.name.clone());
            }
            i += 1;
            keep
        });
        // Record the razing on the age just sealed into the chronicle.
        if let Some(record) = world.era_history.last_mut() {
            record.wonders_razed = fallen.len() as u32;
        }
        for name in fallen {
            world.chronicle.push(
                world.year,
                EventKind::Region,
                fill(
                    &data.strings.chronicle.landmark_razed,
                    &[("landmark", name)],
                ),
            );
        }
    }

    // A new age wipes the transient overlays of the old (GDD 5.7 <-> 5.6/5.3/5.2):
    // the skies clear, and the pestilence and beasts that stalked the closing age
    // pass with it. The persistent world — its regions, heroes, and towns —
    // carries over transformed, but these per-region afflictions reset, as the
    // weather always has, so a plague or beast never outlives the age it was born
    // in unremarked. The sweep is chronicled only when there was something to
    // sweep.
    let afflictions_swept = !world.plagues.is_empty() || !world.monsters.is_empty();
    world.weather.clear();
    world.plagues.clear();
    world.monsters.clear();
    if afflictions_swept {
        world.chronicle.push(
            world.year,
            EventKind::System,
            data.strings.chronicle.age_sweeps_afflictions.clone(),
        );
    }

    // A new era dawns, named after the trigger that ended the last — its cause
    // written into its name (GDD 5.7). `dominant_trigger` still holds the closing
    // age's cause here; it is recomputed next tick.
    world.era.number += 1;
    world.era.name = generate_era_name(
        &data.era_names,
        Some(world.era.dominant_trigger),
        &mut world.rng,
    );
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
                ("lost", heroes_lost.to_string()),
                ("risen", count.to_string()),
            ],
        ),
    );
}

fn reincarnate_age(rng: &mut macroquad_toolkit::rng::SeededRng, min: u32, max: u32) -> u32 {
    let span = (max - min + 1).max(1) as usize;
    min + rng.below(span) as u32
}

/// A proper "Given Surname" name for a hero born during play, drawn from the hero
/// name bank so an era's heirs read like the seeded roster rather than a string of
/// epithets (GDD 5.4).
fn descendant_name(
    bank: &crate::data::HeroNameBank,
    rng: &mut macroquad_toolkit::rng::SeededRng,
) -> String {
    let first = rng.choose(&bank.first_names).cloned().unwrap_or_default();
    let surname = rng.choose(&bank.surnames).cloned().unwrap_or_default();
    format!("{first} {surname}").trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldState;

    #[test]
    fn an_heir_is_named_given_and_surname_from_the_bank() {
        // A hero born during play gets a proper "Given Surname" drawn from the
        // hero name bank, so heirs read like the seeded roster (GDD 5.4).
        let data = GameData::load().unwrap();
        let mut rng = macroquad_toolkit::rng::SeededRng::new(3);
        let name = descendant_name(&data.hero_names, &mut rng);
        let parts: Vec<&str> = name.split(' ').collect();
        assert_eq!(
            parts.len(),
            2,
            "an heir's name is a given name and a surname: {name}"
        );
        assert!(
            data.hero_names.first_names.iter().any(|f| f == parts[0]),
            "the given name comes from the bank: {}",
            parts[0]
        );
        assert!(
            data.hero_names.surnames.iter().any(|s| s == parts[1]),
            "the surname comes from the bank: {}",
            parts[1]
        );
    }

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

        // The closed age remembers its toll: at least one heir always rises to
        // meet the new age (GDD 5.7).
        let record = world.era_history.last().unwrap();
        assert!(
            record.heroes_risen >= 1,
            "a transition must rouse at least one heir"
        );
        assert!(
            record.heroes_lost <= world.heroes.len() as u32,
            "the fallen can't exceed the roster"
        );
    }

    #[test]
    fn the_turning_of_an_age_sweeps_away_plague_and_beast() {
        // A plague and a beast that stalked the closing age do not outlive it: the
        // transition wipes them as it wipes the skies, and marks the sweep in the
        // chronicle (GDD 5.7 <-> 5.3/5.2).
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut player = PlayerState::new(&data.config);

        world.plagues.push(crate::world::Plague {
            id: "p".to_owned(),
            name: "The Old Fever".to_owned(),
            region_id: world.regions[0].id.clone(),
            severity: 1.0,
            age: 5,
        });
        world.monsters.push(crate::world::Monster {
            id: "m".to_owned(),
            name: "The Old Wyrm".to_owned(),
            type_id: "shadow_wyrm".to_owned(),
            region_id: world.regions[0].id.clone(),
            ferocity: 2.0,
            age: 5,
            apex: false,
        });

        // Break the age.
        for region in &mut world.regions {
            region.danger = 100.0;
            region.chaos = 100.0;
            region.prosperity = 0.0;
            region.refresh_status(&data.balance.region);
        }
        let era_before = world.era.number;
        tick_era(&mut world, &mut player, &data);

        assert!(world.era.number > era_before, "the age should have turned");
        assert!(
            world.plagues.is_empty() && world.monsters.is_empty(),
            "the new age should sweep away the old plagues and beasts"
        );
        assert!(
            world
                .chronicle
                .iter_newest()
                .any(|e| e.message.contains("sweeps away")),
            "the sweep should be chronicled"
        );
    }

    #[test]
    fn an_ages_end_tolls_the_towns() {
        // The transition's human toll reaches the settlements, not just the
        // heroes (GDD 5.7): every town loses a share of its souls.
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut player = PlayerState::new(&data.config);
        for region in &mut world.regions {
            region.danger = 100.0;
            region.chaos = 100.0;
            region.prosperity = 0.0;
            region.refresh_status(&data.balance.region);
        }
        let before: Vec<(String, f32)> = world
            .settlements
            .iter()
            .map(|s| (s.id.clone(), s.population))
            .collect();

        tick_era(&mut world, &mut player, &data);

        assert!(world.era.number > 1, "the age should have ended");
        for (id, was) in &before {
            let now = world
                .settlements
                .iter()
                .find(|s| &s.id == id)
                .map(|s| s.population)
                .expect("settlements are not removed during the transition itself");
            assert!(now < *was, "the age's end should claim souls from {id}");
        }
    }

    #[test]
    fn a_violent_ages_end_can_raze_wonders() {
        // A raze chance of 1.0 topples every wonder as the age turns (GDD 5.7).
        let mut data = GameData::load().unwrap();
        let a = &mut data.balance.era.aftermath;
        for delta in [
            &mut a.cataclysm,
            &mut a.collapse,
            &mut a.conquest,
            &mut a.rupture,
            &mut a.divine_war,
        ] {
            delta.landmark_raze_chance = 1.0;
        }
        let mut world = WorldState::new(&data);
        let mut player = PlayerState::new(&data.config);
        assert!(!world.landmarks.is_empty(), "the seed world has wonders");
        for region in &mut world.regions {
            region.danger = 100.0;
            region.chaos = 100.0;
            region.prosperity = 0.0;
            region.refresh_status(&data.balance.region);
        }

        tick_era(&mut world, &mut player, &data);

        assert!(world.era.number > 1, "the age should have ended");
        assert!(
            world.landmarks.is_empty(),
            "a raze-1.0 age should throw down every wonder"
        );
        assert!(
            world
                .chronicle
                .iter_newest()
                .any(|e| e.message.contains("thrown down")),
            "a razed wonder should be chronicled"
        );
    }

    #[test]
    fn a_transition_wins_a_wager_on_the_age_ending() {
        use crate::data::BetPredicate;
        use crate::world::Bet;
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut player = PlayerState::new(&data.config);
        for region in &mut world.regions {
            region.danger = 100.0;
            region.chaos = 100.0;
            region.prosperity = 0.0;
            region.refresh_status(&data.balance.region);
        }
        player.bets.push(Bet {
            event_id: "spec-1".to_owned(),
            predicate: BetPredicate::AgeEnds,
            bet_type_name: "The Turning Age".to_owned(),
            target_name: "the present age".to_owned(),
            confidence_name: String::new(),
            stake: 10,
            potential_payout: 25,
            odds: 2.0,
            placed_year: world.year,
            deadline_year: world.year + 50,
            resolved: None,
        });
        let favor_before = player.favor;

        tick_era(&mut world, &mut player, &data);

        // The age ended, so the wager wins and its payout is credited — the era
        // boundary must not force-expire it like an ordinary bet.
        assert_eq!(player.bets[0].resolved, Some(true));
        assert_eq!(player.favor, favor_before + 25);
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
            0.0,
            0.0,
            0.0,
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
            0.0,
            0.0,
            0.0,
            0.0,
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
    fn a_worlds_dominant_culture_shapes_how_its_age_ends() {
        use crate::data::Culture;
        use crate::world::compute_scores;
        let data = GameData::load().unwrap();
        let balance = &data.balance.era;
        let mut world = WorldState::new(&data);

        let scores = |w: &WorldState| {
            compute_scores(
                &w.regions,
                &w.heroes,
                &w.magic_paths,
                100,
                data.config.max_favor,
                0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                balance,
            )
        };
        // Start from a neutral baseline (neither martial nor mystical) so the
        // culture deltas are attributable purely to the swap under test.
        for r in &mut world.regions {
            r.culture = Culture::Scholarly;
        }
        let base = scores(&world);
        // Turn every region martial without touching its stats: conquest pressure
        // rises on character alone, rupture unchanged.
        for r in &mut world.regions {
            r.culture = Culture::Martial;
        }
        let martial = scores(&world);
        assert!(
            martial.conquest > base.conquest,
            "a warlike world should trend toward a Conquest age"
        );

        // A wholly mystical world instead trends toward rupture.
        for r in &mut world.regions {
            r.culture = Culture::Mystical;
        }
        let mystical = scores(&world);
        assert!(
            mystical.rupture > base.rupture,
            "a mystical world should trend toward a Magical Rupture age"
        );
        assert!(
            (mystical.rupture - base.rupture - balance.rupture_mystical_culture).abs() < 0.01,
            "a fully mystical world adds exactly the culture weight to rupture"
        );
    }

    #[test]
    fn a_wrathful_pantheon_drives_toward_divine_war() {
        use crate::world::{compute_scores, pantheon_wrath};
        let data = GameData::load().unwrap();
        let balance = &data.balance.era;
        let mut world = WorldState::new(&data);

        // Calm gods add nothing; a fully-roused pantheon adds exactly its weight.
        let scores = |wrath: f32| {
            compute_scores(
                &world.regions,
                &world.heroes,
                &world.magic_paths,
                100,
                data.config.max_favor,
                0,
                0.0,
                0.0,
                0.0,
                0.0,
                wrath,
                balance,
            )
        };
        let calm = scores(0.0).divine_war;
        let wrathful = scores(1.0).divine_war;
        assert!(
            wrathful > calm,
            "roused gods should drive the world toward a Divine War age"
        );
        assert!(
            (wrathful - calm - balance.divinewar_pantheon).abs() < 0.01,
            "full wrath contributes exactly the pantheon weight"
        );

        // The wrath measure itself: zero at the resting baseline, positive above.
        let target = data.balance.pantheon.drift_target;
        for d in &mut world.pantheon {
            d.pressure = target;
        }
        assert_eq!(pantheon_wrath(&world.pantheon, target), 0.0);
        for d in &mut world.pantheon {
            d.pressure = 100.0;
        }
        assert!(pantheon_wrath(&world.pantheon, target) > 0.9);
    }

    #[test]
    fn secession_momentum_raises_collapse_pressure_and_decays() {
        use crate::world::compute_scores;
        let data = GameData::load().unwrap();
        let balance = &data.balance.era;
        let mut world = WorldState::new(&data);

        let stable = compute_scores(
            &world.regions,
            &world.heroes,
            &world.magic_paths,
            100,
            data.config.max_favor,
            0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            balance,
        );
        let fracturing = compute_scores(
            &world.regions,
            &world.heroes,
            &world.magic_paths,
            100,
            data.config.max_favor,
            0,
            0.0,
            50.0,
            0.0,
            0.0,
            0.0,
            balance,
        );
        assert!(
            fracturing.collapse > stable.collapse,
            "regions fracturing from within should raise Collapse pressure"
        );
        // Secession momentum feeds Collapse, not Conquest — the two ties stay
        // distinct.
        assert!((fracturing.conquest - stable.conquest).abs() < f32::EPSILON);

        world.secession_momentum = 40.0;
        let mut player = PlayerState::new(&data.config);
        tick_era(&mut world, &mut player, &data);
        assert!(world.secession_momentum < 40.0);
    }

    #[test]
    fn a_world_gripped_by_plague_builds_toward_collapse() {
        // A pandemic raises Collapse pressure directly, apart from the prosperity
        // the pestilence drains (GDD 5.7 <-> 5.3). Full affliction adds exactly
        // the plague weight.
        use crate::world::compute_scores;
        let data = GameData::load().unwrap();
        let balance = &data.balance.era;
        let world = WorldState::new(&data);

        let collapse = |plague_ratio: f32, famine_ratio: f32| {
            compute_scores(
                &world.regions,
                &world.heroes,
                &world.magic_paths,
                100,
                data.config.max_favor,
                0,
                0.0,
                0.0,
                plague_ratio,
                famine_ratio,
                0.0,
                balance,
            )
            .collapse
        };

        assert!(
            collapse(1.0, 0.0) > collapse(0.0, 0.0),
            "a plague-gripped world should trend toward a Collapse age"
        );
        assert!(
            (collapse(1.0, 0.0) - collapse(0.0, 0.0) - balance.collapse_plague).abs() < 0.01,
            "a wholly plagued world adds exactly the plague weight to Collapse"
        );
        // Famine is the twin of plague: a world of failed granaries drives toward
        // Collapse the same way, adding exactly its own weight (GDD 5.7 <-> 5.3).
        assert!(
            collapse(0.0, 1.0) > collapse(0.0, 0.0),
            "a famine-gripped world should trend toward a Collapse age"
        );
        assert!(
            (collapse(0.0, 1.0) - collapse(0.0, 0.0) - balance.collapse_famine).abs() < 0.01,
            "a wholly starving world adds exactly the famine weight to Collapse"
        );
    }

    #[test]
    fn a_legend_that_falls_at_a_transition_is_chronicled() {
        // A legend taken by an age's violent end is remembered by name, not just
        // folded into the aggregate toll (GDD 5.4 <-> 5.7).
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut player = PlayerState::new(&data.config);
        for region in &mut world.regions {
            region.danger = 100.0;
            region.chaos = 100.0;
            region.prosperity = 0.0;
            region.refresh_status(&data.balance.region);
        }
        // Make the first hero an aged legend, so it certainly dies this passage.
        let legend_bar = *data.balance.hero.renown.thresholds.last().unwrap();
        world.heroes[0].renown = legend_bar + 10.0;
        world.heroes[0].age = data.balance.era.death_age;
        let legend_name = world.heroes[0].name.clone();

        tick_era(&mut world, &mut player, &data);

        assert!(
            !world
                .heroes
                .iter()
                .any(|h| h.name == legend_name && h.is_alive),
            "the aged legend should have fallen at the transition"
        );
        assert!(
            world
                .chronicle
                .iter_newest()
                .any(|e| e.message.contains(&legend_name) && e.message.contains("legend endures")),
            "a legend's fall at a transition should be chronicled by name"
        );
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

    #[test]
    fn violent_ends_take_more_heroes_and_reshape_the_heirs() {
        use crate::data::EraTrigger;
        let a = GameData::load().unwrap().balance.era.aftermath;
        // A Divine War is a deadlier passage than a Collapse.
        assert!(
            a.get(EraTrigger::DivineWar).death_mult > a.get(EraTrigger::Collapse).death_mult,
            "a divine war should be deadlier than a collapse"
        );
        assert!(
            a.get(EraTrigger::Cataclysm).death_mult > 1.0,
            "a cataclysm should raise mortality above the baseline"
        );
        // A Collapse leaves a depleted world with fewer heirs; a Divine War rouses
        // more heroes to meet the new age.
        assert!(
            a.get(EraTrigger::Collapse).descendant_mult < 1.0,
            "a collapse should leave fewer heirs"
        );
        assert!(
            a.get(EraTrigger::DivineWar).descendant_mult > 1.0,
            "a divine war should rouse more heirs"
        );
    }
}
