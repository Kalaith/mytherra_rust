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
use crate::data::{fill, Culture, FamineBalance, LoreBalance, ResourceOutputs, ResourceType};
use crate::world::{Chronicle, EventKind, Region, ResourceNode, Settlement, WeatherEvent};

#[allow(clippy::too_many_arguments)]
pub fn tick_famine(
    regions: &mut [Region],
    settlements: &mut [Settlement],
    weather: &[WeatherEvent],
    resource_nodes: &[ResourceNode],
    balance: &FamineBalance,
    lore_balance: &LoreBalance,
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

        // The gods answer the faithful: a genuinely hallowed land — resonance well
        // above the neutral baseline — sees its harvest blessed, so faith is the
        // slow deliverance from the famine that first drove the people to prayer
        // (GDD 5.3 <-> 5.1). Only devotion past the floor counts, and the surge a
        // dearth stirs builds over years, so this eases a long famine in a devout
        // land rather than sparing any land its onset. A faithless land is not
        // cursed, merely unblessed — the term never runs negative.
        let blessing = (region.divine_resonance - balance.resonance_blessing_floor).max(0.0)
            * balance.harvest_per_resonance;

        // Strain accrues only past the comfort lines: a calm, tolerably prosperous
        // land farms freely, while war beyond bearing and poverty beyond bearing
        // each spoil the harvest in proportion to how far past the line they run.
        let chaos_strain = (region.chaos - balance.chaos_comfort).max(0.0) * balance.chaos_strain;
        let dearth_strain =
            (balance.prosperity_comfort - region.prosperity).max(0.0) * balance.dearth_strain;

        // The land's fertility this tick: its own regrowth, the yield of its
        // fields and fisheries, lifted by a farming people, a blessing on the
        // devout, and the fair or foul weather, then spoiled by whatever war and
        // want press past what the land can bear.
        let delta = balance.base_regrowth + food_bounty + pastoral + blessing + weather_term
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
            // A learned land loses fewer to the dearth: it knows to store grain,
            // ration, and rotate its fields (GDD 5.6 <-> 5.3).
            let mortality = balance.famine_mortality
                * (1.0 - super::lore::toll_relief(region, lore_balance.famine_mortality_relief));
            for settlement in settlements.iter_mut() {
                if settlement.region_id == region.id {
                    settlement.population = (settlement.population * (1.0 - mortality)).max(0.0);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
