//! Dynamic regional culture (GDD 5.2): each tick every region's five cultures
//! are scored from its heroes, landmarks, resources and settlements, and the
//! dominant culture flips only when a challenger beats the incumbent by the
//! inertia margin. Landmarks also set the region's cultural-influence target.
//! Deterministic: no RNG.

use crate::data::strings::ChronicleText;
use crate::data::{fill, Culture, CultureBalance, HeroRole, ResourceType};
use crate::world::{
    Chronicle, EventKind, Hero, Landmark, Region, ResourceNode, Settlement, TradeRoute,
};
use macroquad_toolkit::math::approach;

#[allow(clippy::too_many_arguments)]
pub fn tick_culture(
    regions: &mut [Region],
    heroes: &[Hero],
    landmarks: &[Landmark],
    resources: &[ResourceNode],
    settlements: &[Settlement],
    trade_routes: &[TradeRoute],
    balance: &CultureBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    for region in regions.iter_mut() {
        let mut scores = [0.0f32; 5];

        for hero in heroes
            .iter()
            .filter(|h| h.is_alive && h.region_id == region.id)
        {
            scores[hero_culture(hero.role).index()] +=
                balance.hero_weight * (1.0 + hero.level as f32 / 20.0);
        }
        let mut landmark_count = 0;
        for landmark in landmarks.iter().filter(|l| l.region_id == region.id) {
            scores[landmark.culture.index()] += balance.landmark_weight * landmark.influence;
            landmark_count += 1;
        }
        for node in resources.iter().filter(|n| n.region_id == region.id) {
            scores[resource_culture(node.resource_type).index()] += balance.resource_weight;
        }
        for settlement in settlements.iter().filter(|s| s.region_id == region.id) {
            scores[Culture::Mercantile.index()] +=
                balance.settlement_weight * (settlement.prosperity / 50.0);
        }
        for route in trade_routes.iter().filter(|t| t.touches(&region.id)) {
            scores[Culture::Mercantile.index()] += balance.trade_weight * route.volume;
        }

        // Flip the dominant culture only past the inertia margin.
        let (top_index, top_score) = scores
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, s)| (i, *s))
            .unwrap_or((region.culture.index(), 0.0));
        let top_culture = Culture::ALL[top_index];
        if top_culture != region.culture
            && top_score >= scores[region.culture.index()] + balance.inertia
        {
            region.culture = top_culture;
            chronicle.push(
                year,
                EventKind::Region,
                fill(
                    &text.culture_shift,
                    &[
                        ("region", region.name.clone()),
                        ("culture", top_culture.label().to_owned()),
                    ],
                ),
            );
        }

        // Cultural influence reverts toward a landmark-density target.
        let target = (balance.influence_base
            + landmark_count as f32 * balance.influence_per_landmark)
            .clamp(0.0, 100.0);
        region.cultural_influence =
            approach(region.cultural_influence, target, balance.influence_rate);
    }
}

fn hero_culture(role: HeroRole) -> Culture {
    match role {
        HeroRole::Warrior => Culture::Martial,
        HeroRole::Mage => Culture::Mystical,
        HeroRole::Scholar => Culture::Scholarly,
        HeroRole::Ranger => Culture::Pastoral,
        HeroRole::Merchant => Culture::Mercantile,
        HeroRole::Cleric => Culture::Mystical,
    }
}

fn resource_culture(kind: ResourceType) -> Culture {
    match kind {
        ResourceType::Farmland | ResourceType::Forest => Culture::Pastoral,
        ResourceType::Mine | ResourceType::Fishery | ResourceType::Quarry => Culture::Mercantile,
        ResourceType::Manaspring => Culture::Mystical,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::world::WorldState;

    #[test]
    fn every_role_maps_to_a_culture_and_merchants_are_mercantile() {
        // A merchant is the only role that feeds Mercantile culture, filling the
        // gap the settlement/trade signals otherwise carried alone.
        assert_eq!(hero_culture(HeroRole::Merchant), Culture::Mercantile);
        assert_eq!(hero_culture(HeroRole::Cleric), Culture::Mystical);
        // The mapping is total over every declared role (would not compile
        // otherwise, but this guards the ALL list too).
        for role in HeroRole::ALL {
            let _ = hero_culture(role);
        }
    }

    #[test]
    fn scholarly_landmark_and_scholar_hold_aldermoor_scholarly() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        // Aldermoor seeds Scholarly, has the Grand Library + a scholar hero;
        // it should stay Scholarly after a tick.
        tick_culture(
            &mut world.regions,
            &world.heroes,
            &world.landmarks,
            &world.resource_nodes,
            &world.settlements,
            &world.trade_routes,
            &data.balance.culture,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        let aldermoor = world.regions.iter().find(|r| r.id == "aldermoor").unwrap();
        assert_eq!(aldermoor.culture, Culture::Scholarly);
    }

    #[test]
    fn culture_flips_when_challenger_clears_inertia() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        // Force Kharzul (has War Cairns + warrior) to a weak culture; martial
        // score should overcome the inertia margin and flip it back.
        if let Some(k) = world.regions.iter_mut().find(|r| r.id == "kharzul") {
            k.culture = Culture::Pastoral;
        }
        tick_culture(
            &mut world.regions,
            &world.heroes,
            &world.landmarks,
            &world.resource_nodes,
            &world.settlements,
            &world.trade_routes,
            &data.balance.culture,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        let kharzul = world.regions.iter().find(|r| r.id == "kharzul").unwrap();
        assert_ne!(kharzul.culture, Culture::Pastoral);
    }

    #[test]
    fn hearthmoor_holds_pastoral_over_a_long_run() {
        // Hearthmoor's rangers, farmland/forest, and Harvest Shrine should keep
        // its Pastoral identity despite the Mercantile pull of its settlements
        // and the Grain Road.
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        for _ in 0..80 {
            tick_culture(
                &mut world.regions,
                &world.heroes,
                &world.landmarks,
                &world.resource_nodes,
                &world.settlements,
                &world.trade_routes,
                &data.balance.culture,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
        }
        let hearthmoor = world.regions.iter().find(|r| r.id == "hearthmoor").unwrap();
        assert_eq!(hearthmoor.culture, Culture::Pastoral);
    }
}
