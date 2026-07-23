//! Per-tick refugee flight (GDD 5.3): when a land grows too perilous to bear —
//! wracked by danger, gripped by plague, or stalked by a beast — its people flee,
//! not only die. Each tick the masses stream from the world's most perilous
//! settlements along the caravan roads to the safest, most prosperous haven they
//! can reach — a region they are trade-linked to — so the threats reshape where
//! people live, not merely thin their numbers, and a land cut off from every safe
//! neighbour has nowhere to run. The population-flow counterpart to trade's
//! wealth-flow, and now routed along the same network. Deterministic: no RNG
//! (peril and havens are read straight from world state).

use crate::data::strings::ChronicleText;
use crate::data::{fill, RefugeeBalance, RegionBalance};
use crate::world::{Chronicle, EventKind, Monster, Plague, Region, Settlement, TradeRoute};

#[allow(clippy::too_many_arguments)]
pub fn tick_refugees(
    settlements: &mut [Settlement],
    regions: &mut [Region],
    plagues: &[Plague],
    monsters: &[Monster],
    routes: &[TradeRoute],
    balance: &RefugeeBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    // How perilous each region is to live in: its danger, plus a weight for a
    // present plague or stalking beast.
    let peril = |region: &Region| {
        let mut p = region.danger;
        if plagues.iter().any(|pl| pl.region_id == region.id) {
            p += balance.plague_peril;
        }
        if monsters.iter().any(|m| m.region_id == region.id) {
            p += balance.monster_peril;
        }
        if region.famine {
            p += balance.famine_peril;
        }
        p
    };

    // Where each perilous land's people flee: not to some distant safest realm
    // they have no way to reach, but to the safest-and-richest region they are
    // trade-linked to — the caravan roads that carry wealth carry the refugees who
    // follow them. A land cut off from every safe neighbour has nowhere to run, and
    // its people can only endure or perish where they stand. Computed once from the
    // tick-start state, ties broken by id so the choice is deterministic.
    let haven_dest: Vec<(String, usize)> = regions
        .iter()
        .filter(|src| peril(src) >= balance.flee_threshold)
        .filter_map(|src| {
            let haven = regions
                .iter()
                .filter(|h| {
                    h.id != src.id
                        && peril(h) < balance.haven_max_peril
                        && routes
                            .iter()
                            .any(|r| r.touches(&src.id) && r.touches(&h.id))
                })
                .max_by(|a, b| {
                    (a.prosperity - a.danger)
                        .total_cmp(&(b.prosperity - b.danger))
                        .then_with(|| a.id.cmp(&b.id))
                })?;
            let dest = largest_settlement_index(settlements, &haven.id)?;
            Some((src.id.clone(), dest))
        })
        .collect();
    if haven_dest.is_empty() {
        return;
    }

    // Shed refugees from every settlement in a perilous region toward its own
    // reachable haven, and gather them there — people move, they don't vanish, so
    // this conserves population, unlike the death toll of plague or beast.
    let mut arrivals = vec![0.0_f32; settlements.len()];
    let mut notable: Vec<(String, String)> = Vec::new();
    for i in 0..settlements.len() {
        let Some(dest) = haven_dest
            .iter()
            .find(|(rid, _)| rid == &settlements[i].region_id)
            .map(|(_, d)| *d)
        else {
            continue; // this settlement's land is safe, or has nowhere to flee
        };
        if dest == i {
            continue;
        }
        let region = regions.iter().find(|r| r.id == settlements[i].region_id);
        let Some(region) = region else { continue };
        let p = peril(region);
        let leaving = settlements[i].population * balance.flee_rate * (p / 100.0).clamp(0.0, 1.0);
        if leaving <= 0.0 {
            continue;
        }
        settlements[i].population = (settlements[i].population - leaving).max(0.0);
        arrivals[dest] += leaving;

        if leaving >= balance.notable_flight {
            notable.push((settlements[i].name.clone(), settlements[dest].name.clone()));
        }
    }

    for (source_name, haven_name) in notable {
        chronicle.push(
            year,
            EventKind::Region,
            fill(
                &text.refugee_flight,
                &[("source", source_name), ("haven", haven_name)],
            ),
        );
    }

    // Gather the refugees at each haven and strain its economy — more mouths than
    // the land was feeding. Because havens are chosen by prosperity, that strain is
    // what eventually cedes haven status to somewhere less crowded, spreading the
    // flow rather than piling every refugee into one city forever.
    for i in 0..settlements.len() {
        let n = arrivals[i];
        if n <= 0.0 {
            continue;
        }
        settlements[i].population += n;
        let region_id = settlements[i].region_id.clone();
        if let Some(haven) = regions.iter_mut().find(|r| r.id == region_id) {
            haven.apply_deltas(-balance.haven_strain * n, 0.0, 0.0, 0.0, region_balance);
        }
    }
}

/// Index of the region's most populous settlement, if any.
fn largest_settlement_index(settlements: &[Settlement], region_id: &str) -> Option<usize> {
    settlements
        .iter()
        .enumerate()
        .filter(|(_, s)| s.region_id == region_id)
        .max_by(|(_, a), (_, b)| a.population.total_cmp(&b.population))
        .map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::world::WorldState;

    fn run(world: &mut WorldState, data: &GameData) {
        tick_refugees(
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
    }

    #[test]
    fn people_flee_a_perilous_land_for_a_safe_haven() {
        // A settlement in a war-torn region sheds people to the safest region's
        // town, and population is conserved across the move (GDD 5.3).
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        // Region 0 is deadly; region 1 is a haven. Everything else neutral-safe.
        for (i, r) in world.regions.iter_mut().enumerate() {
            r.danger = if i == 0 { 90.0 } else { 5.0 };
            r.prosperity = if i == 1 { 90.0 } else { 40.0 };
        }
        let perilous_id = world.regions[0].id.clone();
        let haven_id = world.regions[1].id.clone();

        let total_before: f32 = world.settlements.iter().map(|s| s.population).sum();
        let perilous_before: f32 = world
            .settlements
            .iter()
            .filter(|s| s.region_id == perilous_id)
            .map(|s| s.population)
            .sum();
        let haven_before: f32 = world
            .settlements
            .iter()
            .filter(|s| s.region_id == haven_id)
            .map(|s| s.population)
            .sum();

        run(&mut world, &data);

        let total_after: f32 = world.settlements.iter().map(|s| s.population).sum();
        let perilous_after: f32 = world
            .settlements
            .iter()
            .filter(|s| s.region_id == perilous_id)
            .map(|s| s.population)
            .sum();
        let haven_after: f32 = world
            .settlements
            .iter()
            .filter(|s| s.region_id == haven_id)
            .map(|s| s.population)
            .sum();

        assert!(
            perilous_after < perilous_before,
            "a deadly land should lose people"
        );
        assert!(haven_after > haven_before, "the haven should take them in");
        assert!(
            (total_after - total_before).abs() < 1.0,
            "refugees move, they don't vanish: {total_before} -> {total_after}"
        );
    }

    #[test]
    fn a_land_cut_off_from_the_roads_has_nowhere_to_flee() {
        // A deadly region with no trade road to any safe haven cannot shed its
        // people — they have no way to reach safety and must endure or perish where
        // they stand (GDD 5.3 <-> 5.2).
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        for (i, r) in world.regions.iter_mut().enumerate() {
            r.danger = if i == 0 { 90.0 } else { 5.0 };
            r.prosperity = if i == 1 { 90.0 } else { 40.0 };
        }
        let perilous_id = world.regions[0].id.clone();
        // Sever every road touching the deadly region: it is now an island.
        world.trade_routes.retain(|r| !r.touches(&perilous_id));

        let before: f32 = world
            .settlements
            .iter()
            .filter(|s| s.region_id == perilous_id)
            .map(|s| s.population)
            .sum();
        run(&mut world, &data);
        let after: f32 = world
            .settlements
            .iter()
            .filter(|s| s.region_id == perilous_id)
            .map(|s| s.population)
            .sum();
        assert_eq!(
            before, after,
            "a land cut off from the roads keeps its people — they cannot flee"
        );
    }

    #[test]
    fn a_swollen_haven_pays_the_strain_of_the_influx() {
        // Taking in refugees strains the haven region's prosperity — the brake
        // that keeps one city from swallowing every refugee forever (GDD 5.3).
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        for (i, r) in world.regions.iter_mut().enumerate() {
            r.danger = if i == 0 { 90.0 } else { 5.0 };
            r.prosperity = if i == 1 { 90.0 } else { 40.0 };
        }
        let haven_id = world.regions[1].id.clone();
        let prosperity_before = world.regions[1].prosperity;

        run(&mut world, &data);

        let haven = world.regions.iter().find(|r| r.id == haven_id).unwrap();
        assert!(
            haven.prosperity < prosperity_before,
            "taking in refugees should strain the haven's prosperity"
        );
    }

    #[test]
    fn a_plague_drives_people_out_even_from_a_calm_land() {
        // Peril isn't only danger: a plague pushes a region over the flee
        // threshold that its danger alone wouldn't reach (GDD 5.3).
        let data = GameData::load().unwrap();
        let b = &data.balance.refugee;

        let fled = |plagued: bool| {
            let mut world = WorldState::new(&data);
            // A middling-danger region, safe enough on its own to keep its people.
            for (i, r) in world.regions.iter_mut().enumerate() {
                r.danger = if i == 0 { b.flee_threshold - 10.0 } else { 5.0 };
                r.prosperity = if i == 1 { 90.0 } else { 40.0 };
            }
            let src = world.regions[0].id.clone();
            if plagued {
                world.plagues.push(crate::world::Plague {
                    id: "p".to_owned(),
                    name: "The Fever".to_owned(),
                    region_id: src.clone(),
                    severity: 1.0,
                    age: 1,
                });
            }
            let before: f32 = world
                .settlements
                .iter()
                .filter(|s| s.region_id == src)
                .map(|s| s.population)
                .sum();
            run(&mut world, &data);
            let after: f32 = world
                .settlements
                .iter()
                .filter(|s| s.region_id == src)
                .map(|s| s.population)
                .sum();
            before - after
        };

        assert!(fled(false).abs() < 1e-3, "a safe land keeps its people");
        assert!(
            fled(true) > 0.0,
            "a plague should drive people to flee a land danger alone would not"
        );
    }

    #[test]
    fn with_nowhere_safe_no_one_flees() {
        // If every region is perilous, there is no haven and the population holds
        // where it is — better the peril you know than the road to nowhere.
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        for r in &mut world.regions {
            r.danger = 95.0;
        }
        let before: Vec<f32> = world.settlements.iter().map(|s| s.population).collect();
        run(&mut world, &data);
        let after: Vec<f32> = world.settlements.iter().map(|s| s.population).collect();
        assert_eq!(before, after, "with no haven, no one moves");
    }
}
