//! Per-tick harvest and famine (GDD 5.3): the food economy beneath every other
//! system. Each region keeps a granary — a 0-100 `harvest` stock that fills from
//! the land's fertility (fair weather, prosperity, a farming culture) and empties
//! under chaos and the weight of its people. When a granary runs dry the region
//! tips into famine: restive, poorer, and bleeding its people to safer ground,
//! until the harvest climbs back and the dearth breaks. This is what gives foul
//! weather and long wars a slow, demographic cost, not merely a passing dip in a
//! stat. Deterministic: harvest is read straight from world state — no roll
//! decides a famine, so the seeded stream is untouched.

use crate::data::strings::ChronicleText;
use crate::data::{fill, Culture, FamineBalance, ResourceOutputs, ResourceType};
use crate::world::{Chronicle, EventKind, Region, ResourceNode, Settlement, WeatherEvent};

#[allow(clippy::too_many_arguments)]
pub fn tick_famine(
    regions: &mut [Region],
    settlements: &mut [Settlement],
    weather: &[WeatherEvent],
    resource_nodes: &[ResourceNode],
    balance: &FamineBalance,
    resource_outputs: &ResourceOutputs,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    for region in regions.iter_mut() {
        // The skies over this land, summed: a fair front (net-positive prosperity)
        // feeds the granary, a storm or blight empties it, each scaled by how hard
        // it is blowing.
        let weather_term: f32 = weather
            .iter()
            .filter(|w| w.region_id == region.id)
            .map(|w| w.prosperity * w.magnitude)
            .sum::<f32>()
            * balance.weather_coeff;

        // The fields and the sea: every farmland and fishery in the region feeds
        // the granary in proportion to its health, so a land rich in fertile,
        // flourishing ground resists dearth while one whose fields lie corrupted
        // or spent — the fate war and overwork visit on a node — is left hungry
        // (GDD 5.3). A depleted node yields nothing, its output multiplier zero.
        let food_bounty: f32 = resource_nodes
            .iter()
            .filter(|n| {
                n.region_id == region.id
                    && matches!(
                        n.resource_type,
                        ResourceType::Farmland | ResourceType::Fishery
                    )
            })
            .map(|n| n.output(resource_outputs))
            .sum::<f32>()
            * balance.harvest_per_food_node;

        let pastoral = if region.culture == Culture::Pastoral {
            balance.pastoral_bonus
        } else {
            0.0
        };

        // Strain accrues only past the comfort lines: a calm, tolerably prosperous
        // land farms freely, while war beyond bearing and poverty beyond bearing
        // each spoil the harvest in proportion to how far past the line they run.
        let chaos_strain = (region.chaos - balance.chaos_comfort).max(0.0) * balance.chaos_strain;
        let dearth_strain =
            (balance.prosperity_comfort - region.prosperity).max(0.0) * balance.dearth_strain;

        // The land's fertility this tick: its own regrowth, the yield of its
        // fields and fisheries, lifted by a farming people and blessed or cursed
        // by the weather, then spoiled by whatever war and want press past what
        // the land can bear.
        let delta = balance.base_regrowth + food_bounty + pastoral + weather_term
            - chaos_strain
            - dearth_strain;
        region.harvest = (region.harvest + delta).clamp(0.0, 100.0);

        // Hysteresis so a dearth doesn't flicker on the threshold: it takes hold
        // once the granary runs past the onset floor and lifts only when the
        // harvest has climbed well back toward plenty.
        if region.famine {
            if region.harvest >= balance.relief {
                region.famine = false;
                chronicle.push(
                    year,
                    EventKind::Region,
                    fill(&text.famine_breaks, &[("region", region.name.clone())]),
                );
            }
        } else if region.harvest <= balance.onset {
            region.famine = true;
            chronicle.push(
                year,
                EventKind::Region,
                fill(&text.famine_begins, &[("region", region.name.clone())]),
            );
        }

        // A land in famine starves and seethes: unrest rises, wealth drains, and
        // its towns lose people to hunger — the refugee system sheds still more.
        if region.famine {
            region.chaos = (region.chaos + balance.famine_chaos).clamp(0.0, 100.0);
            region.prosperity = (region.prosperity - balance.famine_prosperity).clamp(0.0, 100.0);
            for settlement in settlements.iter_mut() {
                if settlement.region_id == region.id {
                    settlement.population =
                        (settlement.population * (1.0 - balance.famine_mortality)).max(0.0);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::world::WorldState;

    fn setup() -> (WorldState, GameData) {
        let data = GameData::load().unwrap();
        let world = WorldState::new(&data);
        (world, data)
    }

    #[test]
    fn a_calm_prosperous_land_never_starves() {
        let (mut world, data) = setup();
        let b = &data.balance.famine;
        let region = &mut world.regions[0];
        region.chaos = 20.0;
        region.prosperity = 60.0;
        region.harvest = 60.0;
        region.famine = false;
        for _ in 0..50 {
            tick_famine(
                &mut world.regions,
                &mut world.settlements,
                &[],
                &[],
                b,
                &data.balance.resource.outputs,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
        }
        assert!(
            !world.regions[0].famine,
            "a calm, prosperous land should keep its granaries full"
        );
        assert!(world.regions[0].harvest > b.onset);
    }

    #[test]
    fn a_chaotic_land_starves_then_recovers_when_order_returns() {
        let (mut world, data) = setup();
        let b = &data.balance.famine;
        let idx = 0;
        world.regions[idx].chaos = 95.0;
        world.regions[idx].prosperity = 15.0;
        world.regions[idx].harvest = 40.0;
        world.regions[idx].famine = false;

        // Under chaos and squalor the granary drains until famine takes hold.
        let mut struck = false;
        for _ in 0..200 {
            tick_famine(
                &mut world.regions,
                &mut world.settlements,
                &[],
                &[],
                b,
                &data.balance.resource.outputs,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
            if world.regions[idx].famine {
                struck = true;
                break;
            }
        }
        assert!(struck, "a war-torn, wretched land should eventually starve");

        // Restore order, and the harvest returns and breaks the famine.
        world.regions[idx].chaos = 15.0;
        world.regions[idx].prosperity = 60.0;
        let mut broke = false;
        for _ in 0..200 {
            tick_famine(
                &mut world.regions,
                &mut world.settlements,
                &[],
                &[],
                b,
                &data.balance.resource.outputs,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
            if !world.regions[idx].famine {
                broke = true;
                break;
            }
        }
        assert!(broke, "a recovered land should break its famine");
    }

    #[test]
    fn a_famine_thins_the_towns_it_grips() {
        let (mut world, data) = setup();
        let b = &data.balance.famine;
        let idx = 0;
        let region_id = world.regions[idx].id.clone();
        world.regions[idx].famine = true;
        world.regions[idx].harvest = 5.0;
        world.regions[idx].chaos = 90.0;
        world.regions[idx].prosperity = 10.0;
        let sidx = world
            .settlements
            .iter()
            .position(|s| s.region_id == region_id)
            .expect("seed region has a settlement");
        world.settlements[sidx].population = 10_000.0;
        let before = world.settlements[sidx].population;
        tick_famine(
            &mut world.regions,
            &mut world.settlements,
            &[],
            &[],
            b,
            &data.balance.resource.outputs,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        assert!(
            world.settlements[sidx].population < before,
            "a famine should cost its towns people"
        );
    }

    #[test]
    fn fertile_fields_fill_the_granary_where_barren_land_starves() {
        use crate::data::{ResourceStatus, ResourceType};
        use crate::world::ResourceNode;
        let (world, data) = setup();
        let b = &data.balance.famine;
        let region_id = world.regions[0].id.clone();

        // The one-tick harvest gain a chaos-strained region draws, given its
        // resource nodes; everything else about the region held fixed.
        let gain_with = |nodes: Vec<ResourceNode>| {
            let mut world = world.clone();
            world.resource_nodes = nodes;
            world.regions[0].chaos = 55.0;
            world.regions[0].prosperity = 45.0;
            world.regions[0].harvest = 50.0;
            world.regions[0].famine = false;
            tick_famine(
                &mut world.regions,
                &mut world.settlements,
                &[],
                &world.resource_nodes,
                b,
                &data.balance.resource.outputs,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
            world.regions[0].harvest - 50.0
        };

        let node = |resource_type: ResourceType, status: ResourceStatus| ResourceNode {
            id: "n".to_owned(),
            name: "N".to_owned(),
            region_id: region_id.clone(),
            resource_type,
            status,
        };
        let barren = gain_with(vec![]);
        let farmed = gain_with(vec![
            node(ResourceType::Farmland, ResourceStatus::Flourishing),
            node(ResourceType::Fishery, ResourceStatus::Active),
        ]);
        let spent = gain_with(vec![node(ResourceType::Farmland, ResourceStatus::Depleted)]);
        let mined = gain_with(vec![node(ResourceType::Mine, ResourceStatus::Flourishing)]);
        assert!(
            farmed > barren,
            "fertile fields should feed the granary ({farmed} vs {barren})"
        );
        assert_eq!(
            spent, barren,
            "a depleted field yields nothing to the granary"
        );
        assert_eq!(
            mined, barren,
            "only fields and fisheries feed the granary, not a mine"
        );
    }
}
