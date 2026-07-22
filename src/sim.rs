//! World tick orchestration. The server would own this in the multiplayer
//! design (GDD 7.1); in this local build the client runs it on a timer.

mod artifact;
mod champion;
mod civilization;
mod consequence;
mod culture;
mod era;
mod famine;
mod genesis;
mod hero;
mod house;
mod landmark;
mod magic;
mod monster;
mod myth;
mod pact;
mod pantheon;
mod plague;
mod refugee;
mod region;
mod resource;
mod settlement;
mod speculation;
mod trade;
mod war;
mod weather;

use crate::data::{fill, GameData, PlayerBalance};
use crate::world::{EventKind, Hero, PlayerState, Region, WorldState};

/// Favor the world's faithful lands tithe their god this tick (GDD 5.1 <-> 5.4):
/// each region's divine resonance above the neutral baseline pours a little power
/// back to the deity it serves, summed across the world and floored to whole
/// favor. So a world of hallowed lands — consecrated by the player or tended by
/// its Clerics — sustains more divine action than a faithless one, closing the
/// favor loop. A land at or below the baseline tithes nothing.
pub fn faith_tithe(regions: &[Region], balance: &PlayerBalance) -> i64 {
    let devotion: f32 = regions
        .iter()
        .map(|r| (r.divine_resonance - balance.favor_tithe_baseline).max(0.0))
        .sum();
    (devotion * balance.favor_per_resonance) as i64
}

/// Advance the entire world by one tick: age every region, credit passive
/// favor, and record the chronicle entries a returning player would read.
pub fn tick_world(world: &mut WorldState, player: &mut PlayerState, data: &GameData) {
    world.year += 1;
    world.tick_count += 1;

    // Heroes who were already legends (the top renown title) before this tick, so
    // we can chronicle the moment any hero first crosses that bar — a milestone
    // the level-up, era-survival, and mastered-magic renown systems all feed.
    let legend_bar = data
        .balance
        .hero
        .renown
        .thresholds
        .last()
        .copied()
        .unwrap_or(f32::INFINITY);
    let already_legend: Vec<String> = world
        .heroes
        .iter()
        .filter(|h| h.renown >= legend_bar)
        .map(|h| h.id.clone())
        .collect();

    let mut newly_in_crisis: Vec<String> = Vec::new();
    for region in &mut world.regions {
        // Baseline for this tick's trend arrows, before any system moves stats.
        region.snapshot_trend();
        let was_crisis = region.status.is_crisis();
        region::tick_region(region, &data.balance.region);
        if region.status.is_crisis() && !was_crisis {
            newly_in_crisis.push(region.name.clone());
        }
    }

    settlement::tick_settlements(
        &mut world.settlements,
        &world.buildings,
        &mut world.regions,
        &world.resource_nodes,
        &data.balance.settlement,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        &data.strings.ui.settlement_tiers,
        world.year,
    );

    settlement::tick_settlement_abandonment(
        &mut world.settlements,
        &mut world.buildings,
        &data.balance.settlement,
        &world.regions,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    settlement::tick_settlement_founding(
        &mut world.settlements,
        &world.regions,
        &mut world.settlement_seq,
        &data.settlement_names,
        &data.balance.settlement,
        &mut world.rng,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    settlement::tick_construction(
        &world.settlements,
        &world.regions,
        &mut world.buildings,
        &data.building_types,
        &data.balance.settlement,
        &mut world.rng,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    resource::tick_resources(
        &mut world.resource_nodes,
        &mut world.regions,
        &mut world.rng,
        &data.balance.resource,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    resource::tick_resource_discovery(
        &mut world.resource_nodes,
        &world.regions,
        &mut world.resource_seq,
        &data.balance.resource,
        &mut world.rng,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    trade::tick_trade(
        &world.trade_routes,
        &mut world.regions,
        &world.heroes,
        &world.resource_nodes,
        &data.balance.trade,
        &data.balance.region,
    );

    trade::tick_trade_founding(
        &mut world.trade_routes,
        &world.regions,
        &mut world.trade_seq,
        &data.balance.trade,
        &mut world.rng,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    plague::tick_plague(
        &mut world.plagues,
        &mut world.regions,
        &mut world.settlements,
        &world.heroes,
        &world.trade_routes,
        &mut world.plague_seq,
        &data.plague_names,
        &data.balance.plague,
        &data.balance.region,
        &mut world.rng,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    let beasts_slain = monster::tick_monster(
        &mut world.monsters,
        &mut world.regions,
        &mut world.settlements,
        &mut world.heroes,
        &data.monster_types,
        &mut world.monster_seq,
        &data.balance.monster,
        &data.balance.region,
        &mut world.rng,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
    // A felled beast becomes a legend of the hunt: a Valor tale the player may
    // promote, so the bestiary leaves its mark on the world's folklore and,
    // through it, a land's martial character (GDD 5.2 <-> 5.6).
    for (hero_name, beast_name, region_id) in beasts_slain {
        let region_name = world
            .regions
            .iter()
            .find(|r| r.id == region_id)
            .map(|r| r.name.clone())
            .unwrap_or_default();
        myth::seed_beast_myth(
            &mut world.myth_candidates,
            &mut world.myth_seq,
            &hero_name,
            &beast_name,
            &region_id,
            &region_name,
            data,
        );
    }

    // Like-cultured, trade-linked, peaceable regions swear alliances that cool
    // their chaos and stay each other's hand from war (GDD 5.2).
    pact::tick_pacts(
        &mut world.pacts,
        &mut world.regions,
        &world.trade_routes,
        &world.wars,
        &mut world.pact_seq,
        &data.balance.pact,
        &data.balance.region,
        &mut world.rng,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    // Belligerent regions fall to war, draining and scarring one another —
    // wearing down the loser toward the conquest that may follow (GDD 5.2). Allies
    // are spared each other's swords.
    war::tick_wars(
        &mut world.wars,
        &mut world.regions,
        &mut world.settlements,
        &world.heroes,
        &world.artifacts,
        &world.pacts,
        &mut world.war_seq,
        &data.balance.war,
        &data.balance.region,
        &mut world.rng,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    // The masses flee the perils just tallied — danger, plague, and beast — for
    // the safest haven, reshaping where the world's people live (GDD 5.3).
    // The granaries fill or fail before the people decide whether to flee, so a
    // land newly gripped by famine drives its refugees this same tick (GDD 5.3).
    famine::tick_famine(
        &mut world.regions,
        &mut world.settlements,
        &world.weather,
        &world.resource_nodes,
        &data.balance.famine,
        &data.balance.resource.outputs,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    refugee::tick_refugees(
        &mut world.settlements,
        &mut world.regions,
        &world.plagues,
        &world.monsters,
        &data.balance.refugee,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    culture::tick_culture(
        &mut world.regions,
        &world.heroes,
        &world.landmarks,
        &world.resource_nodes,
        &world.settlements,
        &world.buildings,
        &world.trade_routes,
        &world.myths,
        &world.houses,
        &data.balance.culture,
        &data.balance.region,
        &data.balance.settlement.tier_thresholds,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    landmark::tick_landmark_founding(
        &mut world.landmarks,
        &world.regions,
        &mut world.landmark_seq,
        &data.landmark_names,
        &data.balance.culture,
        &mut world.rng,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    hero::tick_heroes(
        &mut world.heroes,
        &world.regions,
        &world.landmarks,
        &world.settlements,
        &data.balance.settlement.tier_thresholds,
        &mut world.rng,
        &data.balance.hero,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    hero::tick_faith(
        &world.heroes,
        &mut world.regions,
        &world.plagues,
        &data.balance.hero,
    );
    hero::tick_garrison(
        &world.heroes,
        &mut world.regions,
        &data.balance.hero,
        &data.balance.region,
    );

    champion::tick_champions(
        &mut player.champions,
        &mut world.heroes,
        &mut world.regions,
        &data.balance.champion,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    artifact::tick_artifacts(
        &mut world.artifacts,
        &mut world.regions,
        &mut world.pending_consequences,
        &data.balance.artifact,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    // Delayed aftermath steps of past backlashes unfold here (GDD 5.6).
    consequence::tick_consequences(
        &mut world.pending_consequences,
        &mut world.regions,
        &mut world.settlements,
        &mut world.heroes,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    weather::tick_weather(
        &mut world.weather,
        &mut world.regions,
        &mut world.resource_nodes,
        &data.weather_patterns,
        &data.weather_intensities,
        &mut world.rng,
        &data.balance.weather,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
        // Last tick's era pressure (era runs at the end of the tick); it moves
        // slowly, so the skies rage as the age approaches its breaking.
        world.era.pressure,
    );

    magic::tick_magic(
        &mut world.magic_paths,
        &mut world.regions,
        &mut world.heroes,
        &world.artifacts,
        &world.landmarks,
        &world.resource_nodes,
        &data.balance.magic,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    myth::tick_myths(
        &mut world.myths,
        &mut world.myth_candidates,
        &mut world.myth_seq,
        &mut world.regions,
        &mut world.heroes,
        &mut world.rng,
        &mut world.chronicle,
        data,
        world.year,
    );

    civilization::tick_civilization(
        &mut world.civilization,
        &mut world.regions,
        &data.agendas,
        &world.pacts,
        &data.balance.civilization,
        &data.balance.region,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    // With every stat-mover settled for this tick, let the map reshape: regions
    // pushed past breaking fracture into new ones, and strong powers annex
    // collapsed, undefended neighbours (GDD 5.2).
    genesis::tick_genesis(world, data);

    // Snapshot deity tiers so we can chronicle any god that crests into wrath
    // this tick — the pantheon's autonomous stirring is otherwise silent.
    let deity_tiers: Vec<usize> = world
        .pantheon
        .iter()
        .map(|d| d.tier(&data.balance.pantheon))
        .collect();
    pantheon::tick_pantheon(
        &mut world.pantheon,
        &mut world.regions,
        &data.balance.pantheon,
        &data.balance.region,
    );
    for name in pantheon::deities_cresting(&deity_tiers, &world.pantheon, &data.balance.pantheon) {
        world.chronicle.push(
            world.year,
            EventKind::Divine,
            fill(
                &data.strings.chronicle.deity_ascendant,
                &[("deity", name.clone())],
            ),
        );
        // A god crested to the height of wrath is remembered in myth: the age
        // turns the divine gaze into a tale for the player to promote, themed to
        // the deity's own domain (GDD 5.6 pantheon <-> myths).
        if let Some(stat) = world
            .pantheon
            .iter()
            .find(|d| d.name == name)
            .map(|d| d.effect_stat)
        {
            myth::seed_divine_myth(
                &mut world.myth_candidates,
                &mut world.myth_seq,
                &name,
                stat.into(),
                &world.regions,
                data,
            );
        }
    }

    let era_progress = world.era.pressure / data.balance.era.breaking_threshold.max(1.0);
    speculation::tick_speculations(
        &mut world.speculations,
        &mut world.speculation_seq,
        player,
        &world.heroes,
        &world.regions,
        &world.settlements,
        &mut world.chronicle,
        &mut world.rng,
        data,
        world.year,
        world.era.number,
        era_progress,
    );

    era::tick_era(world, player, data);

    let tithe = faith_tithe(&world.regions, &data.balance.player);
    player.recover(tithe, &data.config, &data.balance.player);

    // The chronicle records notable events, not the passing of each year — the
    // year and favor already live in the HUD, so no per-tick heartbeat clutters
    // the Event Log and drowns the deity's own actions (GDD 10).
    let text = &data.strings.chronicle;
    for name in newly_in_crisis {
        world.chronicle.push(
            world.year,
            EventKind::Region,
            fill(&text.crisis, &[("region", name)]),
        );
    }
    // A hero crossing into legend is chronicled, seeded as a myth candidate about
    // them, and — since a legend is the seed of a dynasty — founds a noble house
    // if they don't already carry one (GDD 5.4).
    let new_legends: Vec<(String, String, f32, String, String)> =
        newly_legendary(&already_legend, &world.heroes, legend_bar)
            .into_iter()
            .map(|h| {
                let region_name = world
                    .regions
                    .iter()
                    .find(|r| r.id == h.region_id)
                    .map(|r| r.name.clone())
                    .unwrap_or_default();
                (
                    h.id.clone(),
                    h.name.clone(),
                    h.renown,
                    h.region_id.clone(),
                    region_name,
                )
            })
            .collect();
    for (id, name, renown, region_id, region_name) in new_legends {
        world.chronicle.push(
            world.year,
            EventKind::Hero,
            fill(&text.hero_legend, &[("hero", name.clone())]),
        );
        myth::seed_hero_legend(
            &mut world.myth_candidates,
            &mut world.myth_seq,
            &name,
            &region_id,
            &region_name,
            data,
        );
        house::found_house(
            &mut world.houses,
            &mut world.house_seq,
            &id,
            &name,
            renown,
            &region_id,
            &region_name,
            &data.balance.house,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
    }

    // Houses reconcile their prestige with their living line, and any whose blood
    // has run out and whose fame has faded pass from memory (GDD 5.4).
    house::tick_houses(
        &mut world.houses,
        &world.heroes,
        &world.regions,
        &data.balance.house,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
}

/// Living heroes who have reached `bar` renown this tick but hadn't before, so
/// the crossing into legend is handled exactly once.
fn newly_legendary<'a>(before: &[String], heroes: &'a [Hero], bar: f32) -> Vec<&'a Hero> {
    heroes
        .iter()
        .filter(|h| h.is_alive && h.renown >= bar && !before.iter().any(|id| id == &h.id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faithful_lands_tithe_favor_and_faithless_ones_do_not() {
        let data = GameData::load().unwrap();
        let balance = &data.balance.player;
        let mut world = WorldState::new(&data);
        world.regions.truncate(2);

        // Both at the neutral baseline: no land is faithful, so no tithe.
        for r in &mut world.regions {
            r.divine_resonance = balance.favor_tithe_baseline;
        }
        assert_eq!(
            faith_tithe(&world.regions, balance),
            0,
            "lands at the baseline tithe nothing"
        );

        // One hallowed land pours favor back; a faithless (below-baseline) one adds
        // nothing, never a negative.
        world.regions[0].divine_resonance = balance.favor_tithe_baseline + 100.0;
        world.regions[1].divine_resonance = balance.favor_tithe_baseline - 30.0;
        let expected = (100.0 * balance.favor_per_resonance) as i64;
        assert_eq!(
            faith_tithe(&world.regions, balance),
            expected,
            "only resonance above the baseline tithes, and never below zero"
        );
        assert!(expected > 0, "a hallowed land should tithe real favor");
    }

    #[test]
    fn only_a_fresh_living_crossing_into_legend_is_reported() {
        use crate::data::HeroRole;
        let h = |id: &str, renown: f32, alive: bool| Hero {
            id: id.to_owned(),
            name: format!("{id}-name"),
            role: HeroRole::Warrior,
            region_id: "r".to_owned(),
            level: 1,
            age: 20,
            is_alive: alive,
            renown,
        };
        let heroes = vec![
            h("old", 200.0, true),   // already a legend last tick — no repeat
            h("new", 200.0, true),   // crossed this tick — announced
            h("dead", 200.0, false), // legendary but fallen — no fanfare
            h("mortal", 50.0, true), // below the bar
        ];
        let before = vec!["old".to_owned()];
        let crossed = newly_legendary(&before, &heroes, 180.0);
        let names: Vec<&str> = crossed.iter().map(|h| h.name.as_str()).collect();
        assert_eq!(names, vec!["new-name"]);
    }

    #[test]
    fn tick_advances_year_and_favor() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut player = PlayerState::new(&data.config);
        player.favor = 0;
        let start_year = world.year;

        tick_world(&mut world, &mut player, &data);

        assert_eq!(world.year, start_year + 1);
        assert_eq!(world.tick_count, 1);
        assert_eq!(player.favor, data.config.favor_per_tick);
    }

    #[test]
    fn a_collapsed_region_is_conquered_through_the_full_tick() {
        // End-to-end: with a dominant power next door and no defender, a
        // crisis-stricken region is annexed — and every later tick system copes
        // with the region vanishing mid-tick. This is the integration guard that
        // removing a region from `world.regions` never desyncs the pipeline.
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut player = PlayerState::new(&data.config);
        let start = world.regions.len();

        let loser_id = world.regions[0].id.clone(); // aldermoor, trade-linked to kharzul
        world.regions[1].prosperity = 90.0;
        world.regions[1].population = 40000.0;
        world.regions[1].chaos = 20.0;
        world.regions[1].danger = 20.0;
        world.regions[1].refresh_status(&data.balance.region);
        world.regions[0].prosperity = 8.0;
        world.regions[0].chaos = 90.0;
        world.regions[0].danger = 90.0;
        world.regions[0].population = 3000.0;
        world.regions[0].refresh_status(&data.balance.region);
        // Remove every hero so no defender can arise — and so this determinism
        // guard stays robust as the seeded roster changes over time (conquest
        // itself uses no RNG; it fires purely on the region state set up here).
        world.heroes.clear();
        // Strip the seeded Protection ward so this tests conquest in isolation.
        world.artifacts.retain(|a| a.region_id != loser_id);

        tick_world(&mut world, &mut player, &data);

        assert_eq!(world.regions.len(), start - 1, "no region was conquered");
        assert!(!world.regions.iter().any(|r| r.id == loser_id));
        assert!(world.summary().region_count == start - 1);
    }

    #[test]
    fn a_region_ground_down_by_turmoil_fractures_through_the_full_tick() {
        // End-to-end: a region kept in crisis (as sustained divine corruption or
        // a long war-torn era would) should eventually split into a new region,
        // and the schism should reach the chronicle — all via `tick_world`.
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut player = PlayerState::new(&data.config);
        // Plant a capable would-be founder in the doomed region.
        let doomed = world.regions[0].id.clone();
        let founder = world.heroes[0].id.clone();
        world.heroes[0].region_id = doomed.clone();
        world.heroes[0].level = 20;

        let mut revolted = false;
        for _ in 0..400 {
            // Keep the region violently unstable, the way relentless corruption
            // or a divine-war era would; drift alone would otherwise calm it. Keep
            // its capable founder in place and alive too, so it always has someone
            // to lead the revolt and a defender against being conquered out from
            // under the test.
            if let Some(r) = world.regions.iter_mut().find(|r| r.id == doomed) {
                r.chaos = 95.0;
                r.danger = 95.0;
            }
            if let Some(h) = world.heroes.iter_mut().find(|h| h.id == founder) {
                h.region_id = doomed.clone();
                h.is_alive = true;
                h.level = 20;
            }
            tick_world(&mut world, &mut player, &data);
            // Break on the revolt itself — other genesis events (a frontier
            // founding elsewhere) may grow the map first, but the thing under test
            // is that sustained turmoil sparks a *revolt*.
            if world
                .chronicle
                .iter_newest()
                .any(|e| e.message.contains("revolt"))
            {
                revolted = true;
                break;
            }
        }
        assert!(
            revolted,
            "sustained turmoil never sparked a chronicled revolt"
        );
    }

    #[test]
    fn the_hero_population_survives_the_ages() {
        // A 400-year unmanaged run must keep a living hero population across the
        // era cullings, and at least one hero must reach the top renown title (a
        // living legend) somewhere along the way — the champion/renown/legend web
        // depends on the roster never dwindling to nothing (GDD 5.4). Guards the
        // hero-lifecycle tuning against a regression that starves the world of
        // heroes.
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut player = PlayerState::new(&data.config);
        let bar = *data.balance.hero.renown.thresholds.last().unwrap();
        let mut min_alive = usize::MAX;
        let mut ever_legend = false;
        for _ in 0..400 {
            tick_world(&mut world, &mut player, &data);
            let living = world.heroes.iter().filter(|h| h.is_alive);
            min_alive = min_alive.min(living.clone().count());
            ever_legend |= living.map(|h| h.renown).fold(0.0_f32, f32::max) >= bar;
        }
        assert!(
            min_alive >= 2,
            "the hero roster dwindled to {min_alive} — the world starves of heroes"
        );
        assert!(ever_legend, "no hero ever rose to legend across four ages");
    }

    #[test]
    fn prosperity_settles_into_a_dynamic_range() {
        // With mean-reverting drift, a long unmanaged run should neither climb
        // toward a static utopia (the positive systems stacking on the reversion)
        // nor collapse the whole world. The upper bound is deliberately tighter
        // than the 100 ceiling: it guards against the world re-drifting into a
        // crisis-free paradise as more prosperity-lifting systems are added.
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut player = PlayerState::new(&data.config);
        for _ in 0..250 {
            tick_world(&mut world, &mut player, &data);
        }
        let summary = world.summary();
        assert!(
            summary.avg_prosperity < 88.0,
            "prosperity drifting toward utopia: {}",
            summary.avg_prosperity
        );
        assert!(
            summary.avg_prosperity > 25.0,
            "world collapsed: {}",
            summary.avg_prosperity
        );
        assert!(
            summary.avg_magic < 92.0,
            "magic pinned: {}",
            summary.avg_magic
        );
    }

    #[test]
    fn the_world_stays_coherent_across_many_ages() {
        // Drive the whole pipeline through several era transitions, asserting the
        // world never degenerates: every stat stays finite and in range, no NaN
        // slips in, settlements never go negative, and the map is never emptied.
        // A cross-system regression guard for the deterministic simulation.
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut player = PlayerState::new(&data.config);

        for _ in 0..350 {
            tick_world(&mut world, &mut player, &data);

            assert!(!world.regions.is_empty(), "the map was emptied of regions");
            for r in &world.regions {
                for v in [
                    r.prosperity,
                    r.chaos,
                    r.danger,
                    r.magic_affinity,
                    r.cultural_influence,
                    r.divine_resonance,
                ] {
                    assert!(
                        v.is_finite() && (0.0..=100.0).contains(&v),
                        "region {} stat out of range: {v}",
                        r.id
                    );
                }
                assert!(r.population.is_finite() && r.population >= 0.0);
                assert!(r.strife.is_finite() && r.strife >= 0.0);
            }
            for s in &world.settlements {
                assert!(
                    s.prosperity.is_finite() && (0.0..=100.0).contains(&s.prosperity),
                    "settlement {} prosperity out of range: {}",
                    s.id,
                    s.prosperity
                );
                assert!(s.population.is_finite() && s.population >= 0.0);
            }
            assert!(player.favor >= 0, "favor went negative");

            // Genesis must never mint two regions sharing a name (GDD 5.2).
            let mut names: Vec<&str> = world.regions.iter().map(|r| r.name.as_str()).collect();
            let total = names.len();
            names.sort_unstable();
            names.dedup();
            assert_eq!(
                total,
                names.len(),
                "two regions share a name at year {}",
                world.year
            );
        }
        assert!(world.year >= 350);
    }

    #[test]
    fn the_same_seed_yields_a_bit_identical_world() {
        // GDD 5.8: the simulation is fully deterministic — the same seed and the
        // same inputs must reproduce the exact same world, byte for byte, so any
        // outcome is auditable. Two independent runs are compared over their full
        // serialized state (not just a digest) after many ages.
        let data = GameData::load().unwrap();
        let run = || {
            let mut world = WorldState::new(&data);
            let mut player = PlayerState::new(&data.config);
            for _ in 0..200 {
                tick_world(&mut world, &mut player, &data);
            }
            (
                serde_json::to_string(&world).expect("world serializes"),
                serde_json::to_string(&player).expect("player serializes"),
            )
        };
        assert_eq!(
            run(),
            run(),
            "same seed must reproduce identical world and player state"
        );
    }
}
