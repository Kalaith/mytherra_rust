//! Dynamic regional culture (GDD 5.2): each tick every region's five cultures
//! are scored from its heroes, landmarks, resources and settlements, and the
//! dominant culture flips only when a challenger beats the incumbent by the
//! inertia margin. Landmarks also set the region's cultural-influence target.
//! Deterministic: no RNG.

use crate::data::strings::ChronicleText;
use crate::data::{fill, Culture, CultureBalance, HeroRole, RegionBalance, ResourceType};
use crate::world::{
    Building, Chronicle, EventKind, Hero, House, Landmark, Myth, Order, Region, ResourceNode,
    Saint, Settlement, TradeRoute,
};
use macroquad_toolkit::math::approach;

#[allow(clippy::too_many_arguments)]
pub fn tick_culture(
    regions: &mut [Region],
    heroes: &[Hero],
    landmarks: &[Landmark],
    resources: &[ResourceNode],
    settlements: &[Settlement],
    buildings: &[Building],
    trade_routes: &[TradeRoute],
    myths: &[Myth],
    houses: &[House],
    saints: &[Saint],
    orders: &[Order],
    balance: &CultureBalance,
    region_balance: &RegionBalance,
    tier_thresholds: &[f32],
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
        let mut aura = (0.0, 0.0, 0.0, 0.0);
        for landmark in landmarks.iter().filter(|l| l.region_id == region.id) {
            // A storied wonder pulls harder on its region's culture the longer it
            // has stood (its stature), but radiates the same physical aura as the
            // structure it is (GDD 5.2).
            scores[landmark.culture.index()] +=
                balance.landmark_weight * landmark.influence * landmark.stature;
            landmark_count += 1;
            let (dp, dc, dd, dm) =
                landmark_aura(landmark.culture, landmark.influence * balance.landmark_aura);
            aura = (aura.0 + dp, aura.1 + dc, aura.2 + dd, aura.3 + dm);
        }
        // A notable place radiates its character into the land it stands on.
        region.apply_deltas(aura.0, aura.1, aura.2, aura.3, region_balance);
        for node in resources.iter().filter(|n| n.region_id == region.id) {
            scores[resource_culture(node.resource_type).index()] += balance.resource_weight;
        }
        for settlement in settlements.iter().filter(|s| s.region_id == region.id) {
            // A settlement drives commerce by both its prosperity and its size: a
            // great city is a far stronger mercantile engine than a village of
            // equal wealth (GDD 5.2).
            let urban =
                1.0 + settlement.tier(tier_thresholds) as f32 * balance.settlement_tier_weight;
            scores[Culture::Mercantile.index()] +=
                balance.settlement_weight * (settlement.prosperity / 50.0) * urban;
        }
        for route in trade_routes.iter().filter(|t| t.touches(&region.id)) {
            scores[Culture::Mercantile.index()] += balance.trade_weight * route.volume;
        }
        // A land's living legends shape its character (GDD 5.2 <-> 5.6): each myth
        // reinforces the culture its theme embodies — valor a martial people,
        // wonder a mystical one — the more vividly the more it still echoes.
        for myth in myths.iter().filter(|m| m.region_id == region.id) {
            let vividness = (myth.resonance / 100.0).clamp(0.0, 1.0);
            scores[myth.culture.index()] += balance.myth_weight * vividness;
        }
        // The venerated dead shape a land's character too (GDD 5.2 <-> 5.1): a
        // region that keeps a saint's shrine is a holy place, its people turned
        // toward the mystical, the more so the fresher the devotion still owed —
        // and fading, with the saint's memory, back toward the mundane.
        for saint in saints.iter().filter(|s| s.region_id == region.id) {
            let devotion = (saint.veneration / 100.0).clamp(0.0, 1.0);
            scores[Culture::Mystical.index()] += balance.saint_weight * devotion;
        }
        // A great Order stamps its calling on the lands it reaches (GDD 5.2 <-> 5.4):
        // a region that hosts a chapter — a living member of the Order's calling —
        // leans toward that calling's culture, scaled by the Order's standing, so a
        // Warriors' Order hardens its chapters martial and an Arcane Circle turns
        // them mystical. The institutional counterpart to the pull the members
        // themselves already exert, and a reason a calling grown into a power
        // reshapes the map's character, not only its own ranks.
        for order in orders.iter() {
            let has_chapter = heroes
                .iter()
                .any(|h| h.is_alive && h.role == order.role && h.region_id == region.id);
            if has_chapter {
                scores[hero_culture(order.role).index()] +=
                    order.prestige * balance.order_culture_weight;
            }
        }
        // The works a people raise speak for their character: each building in the
        // region adds to the culture it embodies (a Forge to the martial, a Temple
        // to the mystical), reinforcing the region's identity over the ages.
        for building in buildings {
            let Some(culture) = building.culture else {
                continue;
            };
            let in_region = settlements
                .iter()
                .any(|s| s.id == building.settlement_id && s.region_id == region.id);
            if in_region {
                scores[culture.index()] += balance.building_weight;
            }
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

        // Cultural influence reverts toward a target set by landmark density and
        // the prestige of the noble houses seated here — a land of great wonders
        // and great lords is a renowned one (GDD 5.2 <-> 5.4).
        let house_prestige: f32 = houses
            .iter()
            .filter(|h| h.seat_region_id == region.id)
            .map(|h| h.prestige.max(0.0))
            .sum();
        let target = (balance.influence_base
            + landmark_count as f32 * balance.influence_per_landmark
            + house_prestige * balance.influence_per_house_prestige)
            .clamp(0.0, 100.0);
        region.cultural_influence =
            approach(region.cultural_influence, target, balance.influence_rate);
    }
}

/// The stat deltas (prosperity, chaos, danger, magic) a landmark radiates, by
/// its culture: scholarly and mystical sites deepen the arcane, mercantile and
/// pastoral ones enrich the land, a martial one makes it more perilous.
fn landmark_aura(culture: Culture, amount: f32) -> (f32, f32, f32, f32) {
    match culture {
        Culture::Scholarly | Culture::Mystical => (0.0, 0.0, 0.0, amount),
        Culture::Mercantile | Culture::Pastoral => (amount, 0.0, 0.0, 0.0),
        Culture::Martial => (0.0, 0.0, amount, 0.0),
    }
}

pub(crate) fn hero_culture(role: HeroRole) -> Culture {
    role.kin_culture()
}

/// The archetypal hero role a culture breeds — the inverse of [`hero_culture`],
/// used when a region's dominant culture shapes the heirs born in a new age
/// (GDD 5.7 <-> 5.2). Mystical breeds mages; clerics arise by the free roll.
pub(crate) fn culture_role(culture: Culture) -> HeroRole {
    match culture {
        Culture::Martial => HeroRole::Warrior,
        Culture::Mystical => HeroRole::Mage,
        Culture::Scholarly => HeroRole::Scholar,
        Culture::Pastoral => HeroRole::Ranger,
        Culture::Mercantile => HeroRole::Merchant,
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
    fn the_works_a_people_raise_reinforce_their_culture() {
        // A pastoral region with no other signals, but whose one settlement holds
        // several forges, should harden martial as its works speak for it (GDD 6).
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut region = world.regions[0].clone();
        region.culture = Culture::Pastoral;
        let region_id = region.id.clone();
        let mut regions = vec![region];
        // Prosperity 0 so the settlement lends no mercantile pull of its own,
        // isolating the buildings' contribution.
        let settlements = vec![Settlement {
            id: "s".to_owned(),
            name: "S".to_owned(),
            region_id: region_id.clone(),
            population: 1000.0,
            prosperity: 0.0,
        }];
        let buildings: Vec<Building> = (0..5)
            .map(|i| Building {
                id: format!("f{i}"),
                name: "Forge".to_owned(),
                settlement_id: "s".to_owned(),
                type_id: "forge".to_owned(),
                prosperity_bonus: 0.0,
                culture: Some(Culture::Martial),
                resonance_bonus: 0.0,
                harvest_bonus: 0.0,
                synergy_resource: None,
            })
            .collect();
        let thresholds = &data.balance.settlement.tier_thresholds;
        for _ in 0..3 {
            tick_culture(
                &mut regions,
                &[],
                &[],
                &[],
                &settlements,
                &buildings,
                &[],
                &[],
                &[],
                &[],
                &[],
                &data.balance.culture,
                &data.balance.region,
                thresholds,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
        }
        assert_eq!(
            regions[0].culture,
            Culture::Martial,
            "a land of forges should harden martial"
        );
    }

    #[test]
    fn a_saints_shrine_turns_its_land_toward_the_mystical() {
        use crate::world::Saint;
        // A martial region whose only cultural signals are the shrines of its
        // venerated dead should, given devotion enough to clear the inertia
        // margin, turn mystical — its people drawn toward the holy (GDD 5.2 <-> 5.1).
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut region = world.regions[0].clone();
        region.culture = Culture::Martial;
        let region_id = region.id.clone();
        let mut regions = vec![region];
        let saint = |id: &str| Saint {
            id: id.to_owned(),
            name: "Saint Test".to_owned(),
            hero_id: id.to_owned(),
            region_id: region_id.clone(),
            veneration: 100.0,
            canonized_year: 0,
        };
        let saints = vec![saint("s1"), saint("s2")];
        let thresholds = &data.balance.settlement.tier_thresholds;
        for _ in 0..30 {
            tick_culture(
                &mut regions,
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                &saints,
                &[],
                &data.balance.culture,
                &data.balance.region,
                thresholds,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
        }
        assert_eq!(
            regions[0].culture,
            Culture::Mystical,
            "a land that keeps a saint's shrine should turn to the mystical"
        );
    }

    #[test]
    fn a_great_order_stamps_its_calling_on_its_chapter() {
        use crate::world::{Hero, Order};
        // A pastoral region hosting a chapter of a storied Warriors' Order — a few
        // resident warriors and the institution behind them — should harden martial,
        // where the same handful of warriors without the Order would not (GDD 5.2
        // <-> 5.4).
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut region = world.regions[0].clone();
        region.culture = Culture::Pastoral;
        let region_id = region.id.clone();

        // A single resident warrior — a chapter of one — whose own cultural pull
        // sits below the inertia margin, so only the Order behind them can tip it.
        let heroes: Vec<Hero> = (0..1)
            .map(|i| Hero {
                id: format!("w{i}"),
                name: format!("Warrior {i}"),
                role: HeroRole::Warrior,
                region_id: region_id.clone(),
                level: 1,
                age: 30,
                is_alive: true,
                renown: 0.0,
            })
            .collect();
        let order = Order {
            id: "o".to_owned(),
            name: "the Warriors' Order".to_owned(),
            role: HeroRole::Warrior,
            prestige: 100.0,
            founded_year: 0,
        };

        let mut flips_with = |orders: &[Order]| {
            let mut regions = vec![{
                let mut r = region.clone();
                r.culture = Culture::Pastoral;
                r
            }];
            let thresholds = &data.balance.settlement.tier_thresholds;
            for _ in 0..30 {
                tick_culture(
                    &mut regions,
                    &heroes,
                    &[],
                    &[],
                    &[],
                    &[],
                    &[],
                    &[],
                    &[],
                    &[],
                    orders,
                    &data.balance.culture,
                    &data.balance.region,
                    thresholds,
                    &mut world.chronicle,
                    &data.strings.chronicle,
                    world.year,
                );
            }
            regions[0].culture == Culture::Martial
        };

        assert!(
            flips_with(std::slice::from_ref(&order)),
            "a chapter of a great Order should harden its land toward its calling"
        );
        assert!(
            !flips_with(&[]),
            "the same warriors without the Order behind them should not flip the land"
        );
    }

    #[test]
    fn culture_role_yields_a_role_of_that_culture() {
        // Each culture's archetypal role maps back to that same culture, so heirs
        // born to a land's culture reinforce it.
        for culture in Culture::ALL {
            assert_eq!(hero_culture(culture_role(culture)), culture);
        }
        assert_eq!(culture_role(Culture::Martial), HeroRole::Warrior);
        assert_eq!(culture_role(Culture::Mercantile), HeroRole::Merchant);
    }

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
    fn a_landmark_radiates_its_character_into_its_region() {
        // Kharzul's martial cairns and gates make the land more perilous, while
        // Sylvenmar's mystical groves deepen its magic (GDD 5.2).
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        for r in &mut world.regions {
            if r.id == "kharzul" || r.id == "sylvenmar" {
                r.danger = 40.0;
                r.magic_affinity = 40.0;
            }
        }

        tick_culture(
            &mut world.regions,
            &world.heroes,
            &world.landmarks,
            &world.resource_nodes,
            &world.settlements,
            &world.buildings,
            &world.trade_routes,
            &world.myths,
            &world.houses,
            &[],
            &[],
            &data.balance.culture,
            &data.balance.region,
            &data.balance.settlement.tier_thresholds,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );

        let kharzul = world.regions.iter().find(|r| r.id == "kharzul").unwrap();
        let sylvenmar = world.regions.iter().find(|r| r.id == "sylvenmar").unwrap();
        assert!(
            kharzul.danger > 40.0,
            "martial landmarks should make Kharzul more perilous: {}",
            kharzul.danger
        );
        assert!(
            sylvenmar.magic_affinity > 40.0,
            "mystical landmarks should deepen Sylvenmar's magic: {}",
            sylvenmar.magic_affinity
        );
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
            &world.buildings,
            &world.trade_routes,
            &world.myths,
            &world.houses,
            &[],
            &[],
            &data.balance.culture,
            &data.balance.region,
            &data.balance.settlement.tier_thresholds,
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
            &world.buildings,
            &world.trade_routes,
            &world.myths,
            &world.houses,
            &[],
            &[],
            &data.balance.culture,
            &data.balance.region,
            &data.balance.settlement.tier_thresholds,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        let kharzul = world.regions.iter().find(|r| r.id == "kharzul").unwrap();
        assert_ne!(kharzul.culture, Culture::Pastoral);
    }

    #[test]
    fn a_great_city_pulls_mercantile_where_a_village_would_not() {
        // One settlement of prosperity 80 is the region's only culture signal.
        // A village's commerce is too weak to overcome the flip inertia, but a
        // metropolis of the same wealth is a strong enough mercantile engine to
        // turn a pastoral land over to commerce (GDD 5.2 — urbanization).
        let data = GameData::load().unwrap();
        let thresholds = &data.balance.settlement.tier_thresholds;
        let run = |population: f32| -> Culture {
            let mut world = WorldState::new(&data);
            let mut region = world.regions[0].clone();
            region.culture = Culture::Pastoral;
            let region_id = region.id.clone();
            let mut regions = vec![region];
            let settlements = vec![Settlement {
                id: "c".to_owned(),
                name: "City".to_owned(),
                region_id,
                population,
                prosperity: 80.0,
            }];
            for _ in 0..5 {
                tick_culture(
                    &mut regions,
                    &[],
                    &[],
                    &[],
                    &settlements,
                    &[],
                    &[],
                    &[],
                    &[],
                    &[],
                    &[],
                    &data.balance.culture,
                    &data.balance.region,
                    thresholds,
                    &mut world.chronicle,
                    &data.strings.chronicle,
                    world.year,
                );
            }
            regions[0].culture
        };
        assert_eq!(
            run(2_000.0),
            Culture::Pastoral,
            "a village's commerce is too weak to flip the region"
        );
        assert_eq!(
            run(40_000.0),
            Culture::Mercantile,
            "a metropolis is a strong enough engine of commerce to flip it"
        );
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
                &world.buildings,
                &world.trade_routes,
                &world.myths,
                &world.houses,
                &[],
                &[],
                &data.balance.culture,
                &data.balance.region,
                &data.balance.settlement.tier_thresholds,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
        }
        let hearthmoor = world.regions.iter().find(|r| r.id == "hearthmoor").unwrap();
        assert_eq!(hearthmoor.culture, Culture::Pastoral);
    }

    #[test]
    fn a_great_houses_seat_grows_in_cultural_influence() {
        // A region that is the seat of a prestigious noble house reverts toward a
        // higher cultural-influence target than the same region with none (GDD 5.2
        // <-> 5.4).
        use crate::world::House;
        let data = GameData::load().unwrap();

        let settled_influence = |seat_prestige: Option<f32>| {
            let mut world = WorldState::new(&data);
            world.regions.truncate(1);
            let region_id = world.regions[0].id.clone();
            world.regions[0].cultural_influence = 0.0;
            world.landmarks.clear();
            world.houses.clear();
            if let Some(prestige) = seat_prestige {
                world.houses.push(House {
                    id: "h".to_owned(),
                    name: "The House of Test".to_owned(),
                    seat_region_id: region_id.clone(),
                    founder_name: "Test".to_owned(),
                    member_ids: vec!["founder".to_owned()],
                    prestige,
                });
            }
            for _ in 0..200 {
                tick_culture(
                    &mut world.regions,
                    &[],
                    &world.landmarks,
                    &[],
                    &[],
                    &[],
                    &[],
                    &[],
                    &world.houses,
                    &[],
                    &[],
                    &data.balance.culture,
                    &data.balance.region,
                    &data.balance.settlement.tier_thresholds,
                    &mut world.chronicle,
                    &data.strings.chronicle,
                    world.year,
                );
            }
            world.regions[0].cultural_influence
        };

        assert!(
            settled_influence(Some(300.0)) > settled_influence(None),
            "a region seated by a great house should grow more culturally renowned"
        );
    }

    #[test]
    fn a_lands_living_legends_shape_its_culture() {
        // A region whose only cultural force is a body of martial legend takes up
        // a Martial character, wherever it started (GDD 5.2 <-> 5.6).
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        world.regions.truncate(1);
        let region_id = world.regions[0].id.clone();
        let region_name = world.regions[0].name.clone();
        world.regions[0].culture = Culture::Scholarly; // start off-martial

        let myths: Vec<Myth> = (0..4)
            .map(|i| Myth {
                id: format!("m{i}"),
                title: "A Tale of Valor".to_owned(),
                theme_name: "Valor".to_owned(),
                stat: crate::data::MythStat::Prosperity,
                cultural_effect: 0.0,
                stat_effect: 0.0,
                culture: Culture::Martial,
                region_id: region_id.clone(),
                region_name: region_name.clone(),
                resonance: 100.0,
                echo_cooldown: 1_000_000, // hold them from echoing; test culture only
            })
            .collect();

        // Nothing else speaks for the land — no heroes, resources, or trade.
        for _ in 0..10 {
            tick_culture(
                &mut world.regions,
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                &myths,
                &[],
                &[],
                &[],
                &data.balance.culture,
                &data.balance.region,
                &data.balance.settlement.tier_thresholds,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
        }

        assert_eq!(
            world.regions[0].culture,
            Culture::Martial,
            "a land remembered for valor should grow martial"
        );
    }
}
