//! Per-tick refugee flight (GDD 5.3): when a land grows too perilous to bear —
//! wracked by danger, gripped by plague, or stalked by a beast — its people flee,
//! not only die. Each tick the masses stream from the world's most perilous
//! settlements toward its safest, most prosperous haven, so the threats reshape
//! where people live, not merely thin their numbers. The population-flow
//! counterpart to trade's wealth-flow. Deterministic: no RNG (peril and haven are
//! read straight from world state).

use crate::data::strings::ChronicleText;
use crate::data::{fill, RefugeeBalance, RegionBalance};
use crate::world::{Chronicle, EventKind, Monster, Plague, Region, Settlement};

#[allow(clippy::too_many_arguments)]
pub fn tick_refugees(
    settlements: &mut [Settlement],
    regions: &mut [Region],
    plagues: &[Plague],
    monsters: &[Monster],
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
        p
    };

    // The haven the masses flee toward: the safest-and-richest region below the
    // haven peril ceiling, ties broken by id so the choice is deterministic. With
    // nowhere safe to run, no one flees this tick.
    let Some(haven_region) = regions
        .iter()
        .filter(|r| peril(r) < balance.haven_max_peril)
        .max_by(|a, b| {
            (a.prosperity - a.danger)
                .total_cmp(&(b.prosperity - b.danger))
                .then_with(|| a.id.cmp(&b.id))
        })
    else {
        return;
    };
    let haven_id = haven_region.id.clone();
    let Some(dest) = largest_settlement_index(settlements, &haven_id) else {
        return; // the haven has no town to take them in
    };

    // Shed refugees from every settlement in a perilous region (never the haven
    // itself), and gather them at the haven — people move, they don't vanish, so
    // this conserves population, unlike the death toll of plague or beast.
    let mut arrivals = 0.0;
    for i in 0..settlements.len() {
        if i == dest {
            continue;
        }
        let Some(region) = regions.iter().find(|r| r.id == settlements[i].region_id) else {
            continue;
        };
        let p = peril(region);
        if p < balance.flee_threshold {
            continue;
        }
        let leaving = settlements[i].population * balance.flee_rate * (p / 100.0).clamp(0.0, 1.0);
        if leaving <= 0.0 {
            continue;
        }
        settlements[i].population = (settlements[i].population - leaving).max(0.0);
        arrivals += leaving;

        if leaving >= balance.notable_flight {
            let source_name = settlements[i].name.clone();
            let haven_name = settlements[dest].name.clone();
            chronicle.push(
                year,
                EventKind::Region,
                fill(
                    &text.refugee_flight,
                    &[("source", source_name), ("haven", haven_name)],
                ),
            );
        }
    }

    settlements[dest].population += arrivals;

    // Taking in the masses strains the haven's economy — more mouths than the
    // land was feeding — and, since havens are chosen by prosperity, that strain
    // is what eventually spreads the flow to somewhere less crowded rather than
    // piling every refugee into one city forever.
    if arrivals > 0.0 {
        if let Some(haven) = regions.iter_mut().find(|r| r.id == haven_id) {
            haven.apply_deltas(
                -balance.haven_strain * arrivals,
                0.0,
                0.0,
                0.0,
                region_balance,
            );
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
