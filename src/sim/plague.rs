//! Per-tick pestilence (GDD 5.3): plagues break out where squalor meets a
//! crowd, sap their region's people and wealth while raising its peril, spread
//! along the trade network the way wealth and ideas do, and burn out as the sick
//! die or recover — soonest where the land is prosperous enough to tend them.
//! The dark counterweight to the world's growth systems. Randomness (outbreak,
//! spread) flows through the world RNG.

use crate::data::strings::ChronicleText;
use crate::data::{fill, PlagueBalance, RegionBalance};
use crate::world::{Chronicle, EventKind, Plague, Region, Settlement, TradeRoute};
use macroquad_toolkit::rng::SeededRng;

#[allow(clippy::too_many_arguments)]
pub fn tick_plague(
    plagues: &mut Vec<Plague>,
    regions: &mut [Region],
    settlements: &mut [Settlement],
    routes: &[TradeRoute],
    seq: &mut u64,
    names: &[String],
    balance: &PlagueBalance,
    region_balance: &RegionBalance,
    rng: &mut SeededRng,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    spawn_outbreaks(
        plagues, regions, seq, names, balance, rng, chronicle, text, year,
    );

    // The toll: each active plague ages a tick, saps its region, and decays as
    // immunity builds — faster where the land is prosperous enough to tend its
    // sick.
    for plague in plagues.iter_mut() {
        plague.age += 1;
        if let Some(region) = regions.iter_mut().find(|r| r.id == plague.region_id) {
            region.apply_deltas(
                -balance.toll_prosperity * plague.severity,
                0.0,
                balance.toll_danger * plague.severity,
                0.0,
                region_balance,
            );
            let recovery = balance.decay_base + region.prosperity * balance.decay_prosperity_coeff;
            plague.severity -= recovery;
        } else {
            // The afflicted region has vanished (conquered, sundered); let the
            // plague gutter out.
            plague.severity -= balance.decay_base;
        }
        // The demographic toll falls on the region's largest settlement.
        if let Some(settlement) = largest_settlement(settlements, &plague.region_id) {
            let loss = settlement.population * balance.toll_population * plague.severity;
            settlement.population = (settlement.population - loss).max(0.0);
        }
    }

    spread_along_roads(plagues, routes, seq, balance, rng);

    // Plagues worn below the severity floor have burned out; chronicle each as it
    // passes and free the region for a future outbreak.
    plagues.retain(|p| {
        if p.severity < balance.min_severity {
            let region_name = regions
                .iter()
                .find(|r| r.id == p.region_id)
                .map(|r| r.name.clone())
                .unwrap_or_else(|| p.region_id.clone());
            chronicle.push(
                year,
                EventKind::Region,
                fill(
                    &text.plague_fades,
                    &[("plague", p.name.clone()), ("region", region_name)],
                ),
            );
            false
        } else {
            true
        }
    });
}

/// Break out fresh plagues in crowded, squalid regions that have none (GDD 5.3).
#[allow(clippy::too_many_arguments)]
fn spawn_outbreaks(
    plagues: &mut Vec<Plague>,
    regions: &[Region],
    seq: &mut u64,
    names: &[String],
    balance: &PlagueBalance,
    rng: &mut SeededRng,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    if names.is_empty() {
        return;
    }
    for region in regions {
        if region.population < balance.outbreak_min_population
            || plagues.iter().any(|p| p.region_id == region.id)
        {
            continue;
        }
        // Squalor breeds pestilence: prosperity below the squalor line raises the
        // chance, the more so the deeper the destitution.
        let squalor = (balance.squalor_prosperity - region.prosperity).max(0.0);
        let chance = balance.outbreak_chance + squalor * balance.squalor_coeff;
        if !rng.chance(chance) {
            continue;
        }

        *seq += 1;
        let pestilence = names[rng.below(names.len())].clone();
        let name = fill(
            &text.plague_name,
            &[("pestilence", pestilence), ("region", region.name.clone())],
        );
        plagues.push(Plague {
            id: format!("plague-{seq}"),
            name: name.clone(),
            region_id: region.id.clone(),
            severity: balance.start_severity,
            age: 0,
        });
        chronicle.push(
            year,
            EventKind::Region,
            fill(
                &text.plague_outbreak,
                &[("plague", name), ("region", region.name.clone())],
            ),
        );
    }
}

/// Let active plagues leap along the trade roads to connected, unafflicted
/// regions (GDD 5.3 <-> 5.2): contagion travels the same network as wealth.
fn spread_along_roads(
    plagues: &mut Vec<Plague>,
    routes: &[TradeRoute],
    seq: &mut u64,
    balance: &PlagueBalance,
    rng: &mut SeededRng,
) {
    // Regions already gripped this tick — a plague never leaps onto a land that
    // is (or is about to be) afflicted, so contagion fans out rather than piling.
    let mut afflicted: Vec<String> = plagues.iter().map(|p| p.region_id.clone()).collect();
    let mut spawned: Vec<Plague> = Vec::new();

    // Snapshot the parents so newly-spread plagues can't themselves spread this
    // same tick (a plague spreads the tick after it arrives, not the instant it
    // lands).
    let parents: Vec<(String, f32, String)> = plagues
        .iter()
        .map(|p| (p.region_id.clone(), p.severity, p.name.clone()))
        .collect();

    for (region_id, severity, name) in parents {
        if !rng.chance(balance.spread_chance) {
            continue;
        }
        // Of the connected regions not yet afflicted, the disease takes the one
        // with the lowest id — an arbitrary but deterministic choice, since the
        // roads themselves don't rank their ends.
        let mut target: Option<String> = None;
        for route in routes {
            let neighbour = if route.region_a == region_id {
                Some(&route.region_b)
            } else if route.region_b == region_id {
                Some(&route.region_a)
            } else {
                None
            };
            if let Some(n) = neighbour {
                let take = match &target {
                    None => true,
                    Some(t) => n.as_str() < t.as_str(),
                };
                if take && !afflicted.iter().any(|a| a == n) {
                    target = Some(n.clone());
                }
            }
        }
        let Some(target) = target else {
            continue;
        };

        *seq += 1;
        afflicted.push(target.clone());
        spawned.push(Plague {
            id: format!("plague-{seq}"),
            // The same pestilence, now abroad in a new land — renamed on arrival
            // would need the region's name, which routes don't carry; keep the
            // parent's name so the chronicle can trace the contagion's path.
            name,
            region_id: target,
            severity: severity * balance.spread_severity_fraction,
            age: 0,
        });
    }

    plagues.extend(spawned);
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

    #[allow(clippy::too_many_arguments)]
    fn run(world: &mut WorldState, data: &GameData, balance: &PlagueBalance) {
        tick_plague(
            &mut world.plagues,
            &mut world.regions,
            &mut world.settlements,
            &world.trade_routes,
            &mut world.plague_seq,
            &data.plague_names,
            balance,
            &data.balance.region,
            &mut world.rng,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
    }

    #[test]
    fn squalor_and_crowding_breed_a_plague() {
        // A crowded, destitute region should eventually take a plague, while an
        // identical but prosperous one stays far healthier (GDD 5.3).
        let data = GameData::load().unwrap();
        let outbreaks = |prosperity: f32| {
            let mut world = WorldState::new(&data);
            world.regions.truncate(1);
            world.regions[0].population = data.balance.plague.outbreak_min_population + 5000.0;
            world.regions[0].prosperity = prosperity;
            let mut count = 0;
            for _ in 0..400 {
                world.plagues.clear(); // isolate outbreak odds, not persistence
                run(&mut world, &data, &data.balance.plague);
                count += world.plagues.len();
            }
            count
        };
        assert!(
            outbreaks(10.0) > outbreaks(95.0),
            "squalor should breed far more plague than plenty"
        );
    }

    #[test]
    fn a_sparse_region_stays_healthy() {
        // Below the crowding floor, no plague takes hold however squalid.
        let data = GameData::load().unwrap();
        let mut balance = data.balance.plague.clone();
        balance.outbreak_chance = 1.0; // would fire every tick if eligible
        let mut world = WorldState::new(&data);
        world.regions.truncate(1);
        world.regions[0].population = balance.outbreak_min_population - 1.0;
        world.regions[0].prosperity = 0.0;

        run(&mut world, &data, &balance);
        assert!(
            world.plagues.is_empty(),
            "a thinly-peopled land breeds no epidemic"
        );
    }

    #[test]
    fn a_plague_saps_its_regions_settlement_and_wealth() {
        let data = GameData::load().unwrap();
        let mut balance = data.balance.plague.clone();
        balance.outbreak_chance = 0.0; // no new outbreaks; study the one we plant
        balance.spread_chance = 0.0;
        let mut world = WorldState::new(&data);
        let region_id = world.regions[0].id.clone();
        world.regions[0].prosperity = 60.0;
        let sidx = world
            .settlements
            .iter()
            .enumerate()
            .filter(|(_, s)| s.region_id == region_id)
            .max_by(|(_, a), (_, b)| a.population.total_cmp(&b.population))
            .map(|(i, _)| i)
            .expect("region has a settlement");
        let pop_before = world.settlements[sidx].population;
        let prosperity_before = world.regions[0].prosperity;
        world.plagues.push(Plague {
            id: "p".to_owned(),
            name: "The Test Fever".to_owned(),
            region_id,
            severity: 2.0,
            age: 0,
        });

        run(&mut world, &data, &balance);

        assert!(
            world.settlements[sidx].population < pop_before,
            "a plague should sap the settlement's people"
        );
        assert!(
            world.regions[0].prosperity < prosperity_before,
            "a plague should drag down its region's prosperity"
        );
    }

    #[test]
    fn a_plague_leaps_along_a_trade_road() {
        // A plague in an isolated region can't spread; one on the trade network
        // leaps to a connected neighbour (GDD 5.3 <-> 5.2).
        let data = GameData::load().unwrap();
        let mut balance = data.balance.plague.clone();
        balance.outbreak_chance = 0.0;
        balance.spread_chance = 1.0; // certain to leap if a road allows it
        balance.decay_base = 0.0; // keep the parent alive across the tick
        balance.decay_prosperity_coeff = 0.0;

        let mut world = WorldState::new(&data);
        // The Iron Road ties aldermoor <-> kharzul.
        world.plagues.clear();
        world.plagues.push(Plague {
            id: "p".to_owned(),
            name: "The Iron Fever".to_owned(),
            region_id: "aldermoor".to_owned(),
            severity: 2.0,
            age: 0,
        });

        run(&mut world, &data, &balance);

        assert!(
            world.plagues.iter().any(|p| p.region_id != "aldermoor"),
            "the plague should have leapt to a connected region"
        );
    }

    #[test]
    fn a_prosperous_land_throws_off_a_plague_sooner() {
        // The same plague fades faster in a rich region than a poor one, because
        // wealth tends the sick (GDD 5.3).
        let data = GameData::load().unwrap();
        let mut balance = data.balance.plague.clone();
        balance.outbreak_chance = 0.0;
        balance.spread_chance = 0.0;

        let ticks_to_fade = |prosperity: f32| {
            let mut world = WorldState::new(&data);
            let region_id = world.regions[0].id.clone();
            world.plagues.push(Plague {
                id: "p".to_owned(),
                name: "The Test Fever".to_owned(),
                region_id,
                severity: 3.0,
                age: 0,
            });
            let mut ticks = 0;
            while !world.plagues.is_empty() && ticks < 1000 {
                // Hold prosperity fixed so only the decay coefficient differs.
                world.regions[0].prosperity = prosperity;
                run(&mut world, &data, &balance);
                ticks += 1;
            }
            ticks
        };

        assert!(
            ticks_to_fade(95.0) < ticks_to_fade(5.0),
            "a wealthy land should throw off a plague sooner than a destitute one"
        );
    }
}
