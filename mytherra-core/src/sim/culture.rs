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
mod tests;
