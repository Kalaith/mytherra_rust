//! Per-tick great Orders (GDD 5.4): the world's professional fellowships, the
//! institutional counterpart to the hereditary House. Where a House is a
//! bloodline seated in one region, an Order is a calling spanning every region
//! its kind dwell in. An Order is founded when a role reaches a critical mass of
//! the living across the world; its prestige drifts toward the size of its
//! fellowship; and it lends cultural weight to each region that hosts a chapter —
//! a member of its calling — until its ranks thin and it disbands. Deterministic:
//! founding, prestige, and effect are all read straight from the roster, no RNG.

use crate::data::strings::{ChronicleText, OrderNames};
use crate::data::{fill, HeroRole, OrderBalance};
use crate::world::{Chronicle, EventKind, Hero, Order, Region};
use macroquad_toolkit::math::approach;

/// Living heroes of a given calling, wherever they dwell — the Order's fellowship.
fn living_members(heroes: &[Hero], role: HeroRole) -> usize {
    heroes
        .iter()
        .filter(|h| h.is_alive && h.role == role)
        .count()
}

#[allow(clippy::too_many_arguments)]
pub fn tick_orders(
    orders: &mut Vec<Order>,
    regions: &mut [Region],
    heroes: &mut [Hero],
    seq: &mut u64,
    balance: &OrderBalance,
    names: &OrderNames,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    // A calling grown numerous enough across the world raises its Order — one to a
    // calling, so a fellowship becomes an institution only once.
    for role in HeroRole::ALL {
        if living_members(heroes, role) >= balance.found_min_members
            && !orders.iter().any(|o| o.role == role)
        {
            *seq += 1;
            let name = names.for_role(role).to_owned();
            orders.push(Order {
                id: format!("order-{seq}"),
                name: name.clone(),
                role,
                prestige: 0.0,
                founded_year: year,
            });
            chronicle.push(
                year,
                EventKind::Hero,
                fill(&text.order_founded, &[("order", name)]),
            );
        }
    }

    // Each Order's standing drifts toward the size of its fellowship; one worn too
    // thin to endure disbands and is chronicled as it passes.
    orders.retain_mut(|order| {
        let members = living_members(heroes, order.role);
        if members < balance.dissolve_min_members {
            chronicle.push(
                year,
                EventKind::Hero,
                fill(&text.order_dissolved, &[("order", order.name.clone())]),
            );
            return false;
        }
        let target = (members as f32 * balance.prestige_per_member).min(balance.prestige_cap);
        order.prestige = approach(order.prestige, target, balance.prestige_rate);
        true
    });

    // Each surviving Order lends its cultural weight to every region that hosts a
    // chapter — a member of its calling — so an institution makes prominent the
    // lands it reaches, the more so the greater its standing.
    for order in orders.iter() {
        let boon = order.prestige * balance.influence_per_prestige;
        if boon <= 0.0 {
            continue;
        }
        for region in regions.iter_mut() {
            let has_chapter = heroes
                .iter()
                .any(|h| h.is_alive && h.role == order.role && h.region_id == region.id);
            if has_chapter {
                region.add_cultural_influence(boon);
            }
        }
    }

    // Belonging to a great Order is itself a distinction: every living member
    // gains renown scaled by the Order's standing, wherever they dwell, so a
    // storied fellowship speeds its own toward legend (GDD 5.4). This threads the
    // institutional layer into the renown web the level-up, era, magic, and myth
    // systems all feed — a young Order lends little, a famed one much.
    for order in orders.iter() {
        let honor = order.prestige * balance.renown_per_prestige;
        if honor <= 0.0 {
            continue;
        }
        for hero in heroes.iter_mut() {
            if hero.is_alive && hero.role == order.role {
                hero.renown += honor;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::world::WorldState;

    /// Build a roster of `n` living heroes of one role, each in region `region`.
    fn roster(n: usize, role: HeroRole, region: &str) -> Vec<Hero> {
        (0..n)
            .map(|i| Hero {
                id: format!("h{i}"),
                name: format!("Hero {i}"),
                role,
                region_id: region.to_owned(),
                level: 3,
                age: 30,
                is_alive: true,
                renown: 0.0,
            })
            .collect()
    }

    fn run(world: &mut WorldState, data: &GameData) {
        tick_orders(
            &mut world.orders,
            &mut world.regions,
            &mut world.heroes,
            &mut world.order_seq,
            &data.balance.order,
            &data.strings.orders,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
    }

    #[test]
    fn a_calling_reaching_critical_mass_founds_its_order() {
        let data = GameData::load().unwrap();
        let b = &data.balance.order;
        let mut world = WorldState::new(&data);
        let region = world.regions[0].id.clone();

        // Just short of the threshold: no Order.
        world.heroes = roster(b.found_min_members - 1, HeroRole::Mage, &region);
        run(&mut world, &data);
        assert!(
            world.orders.is_empty(),
            "a small fellowship founds no Order"
        );

        // At the threshold: the Arcane Circle rises, exactly once.
        world.heroes = roster(b.found_min_members, HeroRole::Mage, &region);
        run(&mut world, &data);
        run(&mut world, &data);
        assert_eq!(
            world.orders.len(),
            1,
            "a calling at critical mass founds its Order, and only one"
        );
        assert_eq!(world.orders[0].role, HeroRole::Mage);
    }

    #[test]
    fn an_orders_prestige_climbs_then_it_disbands_as_its_ranks_thin() {
        let data = GameData::load().unwrap();
        let b = &data.balance.order;
        let mut world = WorldState::new(&data);
        let region = world.regions[0].id.clone();
        world.heroes = roster(b.found_min_members + 2, HeroRole::Warrior, &region);

        for _ in 0..40 {
            run(&mut world, &data);
        }
        assert!(
            world.orders[0].prestige > 0.0,
            "a thriving Order's prestige climbs from nothing"
        );

        // Its fellowship dies away to below the dissolution floor.
        world.heroes.truncate(b.dissolve_min_members - 1);
        run(&mut world, &data);
        assert!(world.orders.is_empty(), "an Order worn too thin disbands");
    }

    #[test]
    fn an_order_lends_cultural_weight_to_its_chapter_regions() {
        let data = GameData::load().unwrap();
        let b = &data.balance.order;
        let mut world = WorldState::new(&data);
        let chapter = world.regions[0].id.clone();
        // Members all dwell in region 0; region 1 hosts none.
        world.heroes = roster(b.found_min_members + 4, HeroRole::Merchant, &chapter);
        world.regions[0].cultural_influence = 50.0;
        world.regions[1].cultural_influence = 50.0;

        for _ in 0..30 {
            run(&mut world, &data);
        }
        assert!(
            world.regions[0].cultural_influence > 50.0,
            "a region hosting a chapter gains cultural influence"
        );
        assert_eq!(
            world.regions[1].cultural_influence, 50.0,
            "a region with no member of the calling gains nothing"
        );
    }

    #[test]
    fn a_storied_order_lends_its_members_renown() {
        let data = GameData::load().unwrap();
        let b = &data.balance.order;
        let mut world = WorldState::new(&data);
        let region = world.regions[0].id.clone();
        // A Mages' Circle and a lone Warrior who belongs to no Order.
        world.heroes = roster(b.found_min_members + 3, HeroRole::Mage, &region);
        world.heroes.push(Hero {
            id: "outsider".to_owned(),
            name: "Unaffiliated".to_owned(),
            role: HeroRole::Warrior,
            region_id: region.clone(),
            level: 3,
            age: 30,
            is_alive: true,
            renown: 0.0,
        });

        for _ in 0..40 {
            run(&mut world, &data);
        }
        let member = world.heroes.iter().find(|h| h.id == "h0").unwrap();
        let outsider = world.heroes.iter().find(|h| h.id == "outsider").unwrap();
        assert!(
            member.renown > 0.0,
            "a member of a storied Order gains renown from the fellowship"
        );
        assert_eq!(
            outsider.renown, 0.0,
            "a hero of a calling with no Order gains no such honor"
        );
    }
}
