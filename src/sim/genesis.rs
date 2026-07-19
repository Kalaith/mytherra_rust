//! Region genesis (GDD 5.2): the world map is not fixed. Three forces reshape
//! it, each in its own sibling module:
//!
//! - [`fracture`] — a region ground down by sustained chaos and danger secedes,
//!   part of it breaking away as a new region under a hero who leads the revolt.
//! - [`conquest`] — a strong region annexes a trade-linked neighbour that has
//!   collapsed into crisis and has no hero to defend it, removing the loser.
//! - [`frontier`] — a veteran hero in a thriving, populous region leads settlers
//!   out to found a new frontier region.
//!
//! The three interact rather than merely coexisting. A high-level hero in a
//! crisis-stricken region *defends* it from conquest but can instead *lead* it
//! to secede — the same catalyst pushing toward fracture and away from being
//! swallowed; the *same* calibre of hero, in a thriving land, instead founds a
//! frontier. Crisis contracts the map (fracture, conquest); success expands it
//! (frontier). Everything is deterministic bar the RNG-drawn breakaway/frontier
//! names, town defections, and the founding roll, so a given seed always
//! reshapes the same way.

mod conquest;
mod fracture;
mod frontier;

use crate::data::GameData;
use crate::world::WorldState;

/// Advance region genesis by one tick: age every region's strife, then let the
/// map reshape — at most one conquest, one founding, and one fracture per tick.
pub fn tick_genesis(world: &mut WorldState, data: &GameData) {
    let WorldState {
        regions,
        settlements,
        resource_nodes,
        landmarks,
        artifacts,
        weather,
        heroes,
        trade_routes,
        civilization,
        region_seq,
        conquest_momentum,
        secession_momentum,
        rng,
        chronicle,
        year,
        ..
    } = world;
    let year = *year;
    let agenda_count = data.agendas.len();
    let region_balance = &data.balance.region;
    let gtext = &data.strings.genesis;
    let ctext = &data.strings.chronicle;

    for region in regions.iter_mut() {
        fracture::accrue_strife(region, &data.balance.genesis);
    }

    conquest::run(
        regions,
        settlements,
        resource_nodes,
        landmarks,
        artifacts,
        weather,
        heroes,
        trade_routes,
        civilization,
        conquest_momentum,
        &data.balance.conquest,
        region_balance,
        chronicle,
        ctext,
        year,
    );

    frontier::run(
        regions,
        heroes,
        civilization,
        region_seq,
        agenda_count,
        rng,
        &data.balance.frontier,
        region_balance,
        chronicle,
        gtext,
        ctext,
        year,
    );

    fracture::run(
        regions,
        settlements,
        heroes,
        civilization,
        region_seq,
        secession_momentum,
        agenda_count,
        rng,
        &data.balance.genesis,
        region_balance,
        chronicle,
        gtext,
        ctext,
        year,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Culture;
    use crate::world::WorldState;

    /// Drive one region deep into turmoil and plant a capable hero there.
    fn primed_world(data: &GameData) -> WorldState {
        let mut world = WorldState::new(data);
        let region = &mut world.regions[0];
        region.chaos = 95.0;
        region.danger = 95.0;
        region.prosperity = 10.0;
        region.refresh_status(&data.balance.region);
        let region_id = region.id.clone();
        world.heroes[0].region_id = region_id;
        world.heroes[0].level = 20;
        world.heroes[0].is_alive = true;
        world
    }

    #[test]
    fn sustained_turmoil_fractures_a_region() {
        let data = GameData::load().unwrap();
        let mut world = primed_world(&data);
        let start = world.regions.len();

        // Keep the region turbulent; strife should cross the threshold and split.
        // The planted level-20 hero also shields it from conquest, so fracture —
        // not annexation — is the outcome.
        let mut fractured = false;
        for _ in 0..200 {
            world.regions[0].chaos = 95.0;
            world.regions[0].danger = 95.0;
            world.regions[0].refresh_status(&data.balance.region);
            tick_genesis(&mut world, &data);
            if world.regions.iter().any(|r| r.id.contains("-rift-")) {
                fractured = true;
                break;
            }
        }
        assert!(fractured, "a region under sustained strife never fractured");
        let child = world
            .regions
            .iter()
            .find(|r| r.id.contains("-rift-"))
            .unwrap();
        assert_eq!(child.culture, Culture::Martial);
        let child_id = child.id.clone();
        assert!(world.regions.len() > start);
        assert!(world.civilization.iter().any(|c| c.region_id == child_id));
        assert!(world.heroes.iter().any(|h| h.region_id == child_id));
        // The secession fed the world's momentum (drives Collapse-era pressure).
        assert!(world.secession_momentum > 0.0);
    }

    #[test]
    fn calm_region_never_reshapes() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let start = world.regions.len();
        // Peaceful, not thriving: no crisis (no fracture/conquest) and short of
        // the thriving bar (no frontier).
        for _ in 0..300 {
            for region in &mut world.regions {
                region.chaos = 20.0;
                region.danger = 20.0;
                region.prosperity = 60.0;
                region.refresh_status(&data.balance.region);
            }
            tick_genesis(&mut world, &data);
        }
        assert_eq!(world.regions.len(), start, "a calm world reshaped itself");
    }

    #[test]
    fn turmoil_without_a_leader_only_builds_pressure() {
        let data = GameData::load().unwrap();
        let mut world = primed_world(&data);
        // Strip every hero of the level that would lead a revolt OR defend a
        // region, and depress the would-be aggressors so conquest cannot fire —
        // proving pressure builds with no genesis event.
        for hero in &mut world.heroes {
            hero.level = 1;
        }
        for region in world.regions.iter_mut().skip(1) {
            region.prosperity = 10.0;
            region.population = 100.0;
        }
        let start = world.regions.len();
        for _ in 0..200 {
            world.regions[0].chaos = 95.0;
            world.regions[0].danger = 95.0;
            world.regions[0].refresh_status(&data.balance.region);
            tick_genesis(&mut world, &data);
        }
        assert_eq!(
            world.regions.len(),
            start,
            "leaderless region still reshaped"
        );
        assert!(
            world.regions[0].strife >= data.balance.genesis.fracture_threshold,
            "pressure should have kept building without a founder"
        );
        assert!(world.regions[0].status.is_crisis());
    }

    #[test]
    fn a_strong_region_conquers_a_defenceless_neighbour() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let start = world.regions.len();

        // aldermoor (idx 0) is trade-linked to kharzul (idx 1).
        let loser_id = world.regions[0].id.clone();
        let winner_id = world.regions[1].id.clone();

        let winner = &mut world.regions[1];
        winner.prosperity = 90.0;
        winner.population = 40000.0;
        winner.chaos = 20.0;
        winner.danger = 20.0;
        winner.refresh_status(&data.balance.region);

        let loser = &mut world.regions[0];
        loser.prosperity = 8.0;
        loser.chaos = 90.0;
        loser.danger = 90.0;
        loser.population = 3000.0;
        loser.refresh_status(&data.balance.region);
        assert!(world.regions[0].status.is_crisis());
        for hero in &mut world.heroes {
            if hero.region_id == loser_id {
                hero.level = 1;
            }
        }

        tick_genesis(&mut world, &data);

        assert_eq!(world.regions.len(), start - 1, "no region was conquered");
        assert!(
            !world.regions.iter().any(|r| r.id == loser_id),
            "the conquered region still exists"
        );
        assert!(
            world
                .settlements
                .iter()
                .filter(|s| s.region_id == winner_id)
                .count()
                >= 1
        );
        assert!(world
            .chronicle
            .iter_newest()
            .any(|e| e.message.contains("absorbs it whole")));
        // The conquest fed the world's momentum (drives Conquest-era pressure).
        assert!(world.conquest_momentum > 0.0);
    }

    #[test]
    fn a_defended_region_resists_conquest() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let start = world.regions.len();
        let loser_id = world.regions[0].id.clone();

        let winner = &mut world.regions[1];
        winner.prosperity = 90.0;
        winner.population = 40000.0;
        winner.chaos = 20.0;
        winner.danger = 20.0;
        winner.refresh_status(&data.balance.region);

        let loser = &mut world.regions[0];
        loser.prosperity = 8.0;
        loser.chaos = 90.0;
        loser.danger = 90.0;
        loser.population = 3000.0;
        loser.refresh_status(&data.balance.region);

        // A lone champion holds the line.
        world.heroes[0].region_id = loser_id.clone();
        world.heroes[0].level = 30;
        world.heroes[0].is_alive = true;

        for _ in 0..5 {
            tick_genesis(&mut world, &data);
        }
        assert_eq!(
            world.regions.len(),
            start,
            "a defended region was conquered anyway"
        );
        assert!(world.regions.iter().any(|r| r.id == loser_id));
    }

    #[test]
    fn a_veteran_in_a_thriving_land_founds_a_frontier() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let start = world.regions.len();

        // A prosperous, populous, stable home with a veteran hero.
        world.heroes[0].region_id = world.regions[0].id.clone();
        world.heroes[0].level = 20;
        world.heroes[0].is_alive = true;

        let mut founded = false;
        for _ in 0..500 {
            let home = &mut world.regions[0];
            home.prosperity = 90.0;
            home.chaos = 10.0;
            home.danger = 10.0;
            home.population = 20000.0;
            home.refresh_status(&data.balance.region);
            // Keep the founder eligible against aging/level drift.
            world.heroes[0].level = 20;
            world.heroes[0].is_alive = true;
            tick_genesis(&mut world, &data);
            if world.regions.iter().any(|r| r.id.contains("-frontier-")) {
                founded = true;
                break;
            }
        }
        assert!(founded, "a thriving land never founded a frontier");
        let frontier = world
            .regions
            .iter()
            .find(|r| r.id.contains("-frontier-"))
            .unwrap();
        let frontier_id = frontier.id.clone();
        assert!(world.regions.len() > start);
        // The frontier carries its own civilization bookkeeping and its founder.
        assert!(world
            .civilization
            .iter()
            .any(|c| c.region_id == frontier_id));
        assert!(world.heroes.iter().any(|h| h.region_id == frontier_id));
        assert!(world
            .chronicle
            .iter_newest()
            .any(|e| e.message.contains("found the frontier")));
    }

    #[test]
    fn a_struggling_land_founds_nothing() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let start = world.regions.len();
        world.heroes[0].region_id = world.regions[0].id.clone();
        world.heroes[0].level = 20;

        // Merely middling — never thriving — so no frontier is founded, and calm
        // enough that no crisis triggers a fracture or conquest either.
        for _ in 0..300 {
            for region in &mut world.regions {
                region.prosperity = 55.0;
                region.chaos = 25.0;
                region.danger = 25.0;
                region.population = 20000.0;
                region.refresh_status(&data.balance.region);
            }
            world.heroes[0].level = 20;
            world.heroes[0].is_alive = true;
            tick_genesis(&mut world, &data);
        }
        assert_eq!(world.regions.len(), start, "a non-thriving land expanded");
    }
}
