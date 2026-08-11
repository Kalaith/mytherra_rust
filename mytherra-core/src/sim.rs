//! World tick orchestration. The server would own this in the multiplayer
//! design (GDD 7.1); in this local build the client runs it on a timer.

pub mod achievements;
mod artifact;
mod champion;
mod civilization;
mod consequence;
mod culture;
mod era;
mod famine;
mod festival;
mod genesis;
mod hero;
mod house;
mod landmark;
mod lore;
mod magic;
mod monster;
mod myth;
mod order;
mod pact;
mod pantheon;
mod plague;
mod prophecy;
mod refugee;
mod region;
mod resource;
mod saint;
mod settlement;
mod speculation;
mod trade;
mod vassalage;
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

/// Advance the entire world by one tick for a single deity — the capture fixture
/// and every determinism test. The one-player case of [`tick_shared`]: aggregates
/// over one player equal that player, so this is byte-identical to the
/// pre-multiplayer tick.
pub fn tick_world(world: &mut WorldState, player: &mut PlayerState, data: &GameData) {
    tick_shared(world, std::slice::from_mut(player), data);
}

/// Advance the shared world by one tick for every connected deity (GDD 7.1). The
/// world itself advances once; each per-deity effect — champions nudging heroes
/// and regions, wagers settling, favor recovering, and a turning age's boundary
/// bets — runs once per player (deterministically, in the slice's order). The
/// server calls this with all its players; single-player calls it with one.
pub fn tick_shared(world: &mut WorldState, players: &mut [PlayerState], data: &GameData) {
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
        // Refill the anti-grief nudge budget: each tick a region can again absorb
        // its cap of divine influence, summed across every deity (GDD 7.5).
        region.refresh_nudge_budget();
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
        &world.weather,
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

    // A civilization's accumulated knowledge creeps toward what its scholars,
    // libraries, mastered magic, and wealth can sustain — the resilience that will
    // soften the plague and dearth to come (GDD 5.6 <-> 5.3).
    lore::tick_lore(
        &mut world.regions,
        &world.heroes,
        &world.landmarks,
        &world.magic_paths,
        &data.balance.lore,
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
        &data.balance.lore,
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
        &world.vassalages,
        &mut world.war_seq,
        &data.balance.war,
        &data.balance.region,
        &mut world.rng,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    // In the space between alliance and annexation, the strong bend the weak:
    // a dominant region takes a far weaker, trade-linked neighbour at peace as a
    // tributary vassal, draining its wealth until it grows strong enough to rebel
    // (GDD 5.2).
    vassalage::tick_vassalages(
        &mut world.vassalages,
        &mut world.regions,
        &world.heroes,
        &world.trade_routes,
        &mut world.vassalage_seq,
        &data.balance.vassalage,
        &data.balance.conquest,
        &data.balance.region,
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
        &data.balance.lore,
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
        &world.trade_routes,
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
        &world.saints,
        &world.orders,
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

    // Each deity's champions nudge the shared heroes and regions, in player
    // order (deterministic). A world with no one connected simply has none.
    for player in players.iter_mut() {
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
    }

    artifact::tick_artifacts(
        &mut world.artifacts,
        &mut world.regions,
        &world.heroes,
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
        &world.vassalages,
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

    prophecy::tick_prophecies(
        &mut world.prophecies,
        &mut world.regions,
        &mut world.prophecy_seq,
        &data.balance.prophecy,
        &data.balance.region,
        &data.strings.prophecies,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
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
    // Resolve the shared board once, settle each deity's wagers against it
    // (before the board is pruned), then refresh it.
    speculation::resolve_events(
        &mut world.speculations,
        &world.heroes,
        &world.regions,
        &world.settlements,
        world.year,
        world.era.number,
    );
    for player in players.iter_mut() {
        speculation::settle_bets(
            &mut player.bets,
            &mut player.favor,
            &world.speculations,
            &mut world.chronicle,
            data,
            world.year,
        );
    }
    speculation::refresh_market(
        &mut world.speculations,
        &mut world.speculation_seq,
        &world.heroes,
        &world.regions,
        &world.settlements,
        &mut world.rng,
        data,
        world.year,
        world.era.number,
        era_progress,
    );

    // Era pressure reads the deities' aggregate favor and pending stake; a
    // turning age then settles each deity's boundary-spanning bets.
    let total_favor: i64 = players.iter().map(|p| p.favor).sum();
    let total_max_favor: i64 = data.config.max_favor * players.len().max(1) as i64;
    let total_pending_stake: i64 = players
        .iter()
        .flat_map(|p| p.bets.iter())
        .filter(|b| b.resolved.is_none())
        .map(|b| b.stake)
        .sum();
    if era::tick_era_world(
        world,
        total_favor,
        total_max_favor,
        total_pending_stake,
        data,
    ) {
        for player in players.iter_mut() {
            era::expire_boundary_bets(&mut player.bets, &mut player.favor);
        }
    }

    let tithe = faith_tithe(&world.regions, &data.balance.player);
    for player in players.iter_mut() {
        player.recover(tithe, &data.config, &data.balance.player);
        // Evaluate each deity's achievements server-side (GDD 7.1): a fresh
        // unlock grants standing experience, so progression works online — the
        // client only ever renders the resulting PlayerView. The client sees the
        // unlock via its polled view; a chronicle line would need a fitting
        // EventKind and authored copy, so it is left to the notification path.
        achievements::check(world, player, data);
    }

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

    order::tick_orders(
        &mut world.orders,
        &mut world.regions,
        &mut world.heroes,
        &mut world.order_seq,
        &data.balance.order,
        &data.strings.orders,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );

    let new_saints = saint::tick_saints(
        &mut world.saints,
        &world.heroes,
        &mut world.regions,
        &mut world.saint_seq,
        &data.balance.saint,
        legend_bar,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
    // A soul raised to sainthood becomes the stuff of legend: a mystical tale of
    // its holiness the player may promote, so the faith layer feeds the world's
    // folklore as the hunt and the passage into legend already do (GDD 5.1 <-> 5.6).
    for (saint_name, region_id, region_name) in new_saints {
        myth::seed_saint_myth(
            &mut world.myth_candidates,
            &mut world.myth_seq,
            &saint_name,
            &region_id,
            &region_name,
            data,
        );
    }

    // The world's great celebrations: once in a generation a flourishing, peaceful
    // realm holds a festival that lifts its culture and faith and crowns its heroes
    // — the constructive mirror of the crises above (GDD 5.2 <-> 6).
    let festivals_remembered = festival::tick_festivals(
        &mut world.festivals,
        &mut world.regions,
        &mut world.heroes,
        &mut world.festival_seq,
        &data.balance.festival,
        &data.strings.festivals,
        &mut world.chronicle,
        &data.strings.chronicle,
        world.year,
    );
    // A festival that passes into memory becomes a Triumph-tale of the realm's
    // golden years, a myth the player may promote — so celebration feeds the
    // world's folklore as the hunt, the saint, and the legend already do (GDD 5.2
    // <-> 6). The land that fetes its splendour grows storied for it.
    for (festival_name, region_id, region_name) in festivals_remembered {
        myth::seed_festival_myth(
            &mut world.myth_candidates,
            &mut world.myth_seq,
            &festival_name,
            &region_id,
            &region_name,
            data,
        );
    }

    // Every subsystem above records into the chronicle in a fixed order, so a busy
    // year's events would otherwise land as blocks of one kind. Weave this tick's
    // events together by kind, once, so the year reads as the mixture it was (GDD
    // 10 — the chronicle as a legible feed).
    world.chronicle.interleave_latest_tick();
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
mod tests;
