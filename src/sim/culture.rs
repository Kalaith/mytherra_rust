//! Dynamic regional culture (GDD 5.2): each tick every region's five cultures
//! are scored from its heroes, landmarks, resources and settlements, and the
//! dominant culture flips only when a challenger beats the incumbent by the
//! inertia margin. Landmarks also set the region's cultural-influence target.
//! Deterministic: no RNG.

use crate::data::strings::ChronicleText;
use crate::data::{fill, Culture, CultureBalance, HeroRole, RegionBalance, ResourceType};
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
            scores[landmark.culture.index()] += balance.landmark_weight * landmark.influence;
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
    match role {
        HeroRole::Warrior => Culture::Martial,
        HeroRole::Mage => Culture::Mystical,
        HeroRole::Scholar => Culture::Scholarly,
        HeroRole::Ranger => Culture::Pastoral,
        HeroRole::Merchant => Culture::Mercantile,
        HeroRole::Cleric => Culture::Mystical,
    }
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
            &world.trade_routes,
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
            &world.trade_routes,
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
            &world.trade_routes,
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
                &world.trade_routes,
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
}
