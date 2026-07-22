//! Per-tick noble houses (GDD 5.4): the great bloodlines the world's legends
//! found. A house arises when a hero passes into legend; its prestige tracks the
//! renown of its living members, and its heirs — born at each age's turn — carry
//! a share of that renown into the world, a head start toward legend of their
//! own. A house whose blood runs out and whose fame has faded is at last
//! forgotten. Prestige and fading are deterministic; heir birth rolls through the
//! world RNG at a transition.

use crate::data::strings::ChronicleText;
use crate::data::{fill, HouseBalance};
use crate::world::{Chronicle, EventKind, Hero, House, Region};

/// Advance every house by one tick: prestige drifts toward the summed renown of
/// its living line, and a house with no blood left and too little standing is
/// forgotten.
pub fn tick_houses(
    houses: &mut Vec<House>,
    heroes: &[Hero],
    regions: &[Region],
    balance: &HouseBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    for house in houses.iter_mut() {
        // A house whose seat has been lost — its region conquered away or sundered
        // by a fracture — follows its blood, reestablishing where its greatest
        // living scion now dwells (GDD 5.4 <-> 5.2). A house with no living blood
        // keeps its lost seat and passes into memory below.
        if !regions.iter().any(|r| r.id == house.seat_region_id) {
            if let Some(new_seat) = greatest_member_home(house, heroes) {
                if new_seat != house.seat_region_id {
                    let region_name = regions
                        .iter()
                        .find(|r| r.id == new_seat)
                        .map(|r| r.name.clone())
                        .unwrap_or_else(|| new_seat.clone());
                    house.seat_region_id = new_seat;
                    chronicle.push(
                        year,
                        EventKind::Hero,
                        fill(
                            &text.house_reseated,
                            &[("house", house.name.clone()), ("region", region_name)],
                        ),
                    );
                }
            }
        }

        let (_, renown) = house_vitality(house, heroes);
        house.prestige += (renown - house.prestige) * balance.prestige_rate;
    }
    houses.retain(|house| {
        let (living, _) = house_vitality(house, heroes);
        if living == 0 && house.prestige < balance.fade_floor {
            chronicle.push(
                year,
                EventKind::Hero,
                fill(&text.house_fades, &[("house", house.name.clone())]),
            );
            false
        } else {
            true
        }
    });
}

/// The region where the house's most renowned living member dwells — the ground
/// a displaced house reestablishes itself upon. Ties break by id.
fn greatest_member_home(house: &House, heroes: &[Hero]) -> Option<String> {
    house
        .member_ids
        .iter()
        .filter_map(|id| heroes.iter().find(|h| &h.id == id))
        .filter(|h| h.is_alive)
        .max_by(|a, b| a.renown.total_cmp(&b.renown).then_with(|| a.id.cmp(&b.id)))
        .map(|h| h.region_id.clone())
}

/// Living-member count and their summed renown for a house.
fn house_vitality(house: &House, heroes: &[Hero]) -> (usize, f32) {
    let mut count = 0;
    let mut renown = 0.0;
    for id in &house.member_ids {
        if let Some(h) = heroes.iter().find(|h| &h.id == id) {
            if h.is_alive {
                count += 1;
                renown += h.renown;
            }
        }
    }
    (count, renown)
}

/// Found a noble house for a hero who has just passed into legend, unless they
/// already belong to one (an heir who rose to legend keeps their birth house).
/// The house is seated in the hero's region and begins with a prestige drawn from
/// the founder's own renown.
#[allow(clippy::too_many_arguments)]
pub fn found_house(
    houses: &mut Vec<House>,
    seq: &mut u64,
    hero_id: &str,
    hero_name: &str,
    hero_renown: f32,
    region_id: &str,
    region_name: &str,
    balance: &HouseBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    if houses.iter().any(|h| h.holds(hero_id)) {
        return;
    }
    *seq += 1;
    let name = fill(&text.house_name, &[("founder", hero_name.to_owned())]);
    houses.push(House {
        id: format!("house-{seq}"),
        name: name.clone(),
        seat_region_id: region_id.to_owned(),
        founder_name: hero_name.to_owned(),
        member_ids: vec![hero_id.to_owned()],
        prestige: hero_renown * balance.found_prestige_fraction,
    });
    chronicle.push(
        year,
        EventKind::Hero,
        fill(
            &text.house_founded,
            &[("house", name), ("region", region_name.to_owned())],
        ),
    );
}

/// Make a newly-born descendant an heir of the noble house seated in their birth
/// region, if one is seated there (GDD 5.4 <-> 5.7): a bloodline renews itself on
/// its ancestral ground, the land's proudest line claiming the notable newborns
/// of its seat. The heir joins the house and inherits a share of its prestige as
/// starting renown — a head start toward legend of their own. Returns the renown
/// the heir is born with (0 if baseborn), and records an heir's birth in the
/// chronicle.
///
/// Deliberately deterministic — it draws no RNG — so the birth of an heir never
/// perturbs the transition's other rolls; the only mark a house leaves on the
/// world is the renown it lends its blood.
pub fn maybe_inherit(
    houses: &mut [House],
    heir_id: &str,
    region_id: &str,
    balance: &HouseBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) -> f32 {
    // The most prestigious house seated in the birth region claims the heir; ties
    // break by id so the choice is fixed.
    let Some(idx) = houses
        .iter()
        .enumerate()
        .filter(|(_, h)| h.seat_region_id == region_id)
        .max_by(|(_, a), (_, b)| {
            a.prestige
                .total_cmp(&b.prestige)
                .then_with(|| a.id.cmp(&b.id))
        })
        .map(|(i, _)| i)
    else {
        return 0.0;
    };

    let house = &mut houses[idx];
    house.member_ids.push(heir_id.to_owned());
    chronicle.push(
        year,
        EventKind::Hero,
        fill(&text.house_heir, &[("house", house.name.clone())]),
    );
    house.prestige * balance.inherit_fraction
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{GameData, HeroRole, HeroSeed};
    use crate::world::WorldState;

    fn legend(id: &str, region_id: &str, renown: f32) -> Hero {
        let mut h = Hero::from_seed(&HeroSeed {
            id: id.to_owned(),
            name: id.to_owned(),
            role: HeroRole::Warrior,
            region_id: region_id.to_owned(),
            level: 30,
            age: 40,
        });
        h.renown = renown;
        h
    }

    #[test]
    fn a_legend_founds_a_house_seated_in_their_land() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        world.houses.clear();
        let region_id = world.regions[0].id.clone();

        found_house(
            &mut world.houses,
            &mut world.house_seq,
            "brogan",
            "Brogan Aldwin",
            200.0,
            &region_id,
            "Aldermoor",
            &data.balance.house,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );

        assert_eq!(world.houses.len(), 1);
        let house = &world.houses[0];
        assert!(house.name.contains("Brogan Aldwin"));
        assert_eq!(house.seat_region_id, region_id);
        assert!(house.holds("brogan"));
        assert_eq!(
            house.prestige,
            200.0 * data.balance.house.found_prestige_fraction
        );

        // A second call for the same hero founds no new house.
        found_house(
            &mut world.houses,
            &mut world.house_seq,
            "brogan",
            "Brogan Aldwin",
            200.0,
            &region_id,
            "Aldermoor",
            &data.balance.house,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        assert_eq!(world.houses.len(), 1, "a hero founds at most one house");
    }

    #[test]
    fn prestige_tracks_the_living_line_and_a_dead_one_fades() {
        let data = GameData::load().unwrap();
        let mut balance = data.balance.house.clone();
        balance.prestige_rate = 1.0; // snap straight to the target for the test
        let mut world = WorldState::new(&data);
        let region_id = world.regions[0].id.clone();

        // A house with one famed living member.
        world.heroes = vec![legend("scion", &region_id, 150.0)];
        world.houses = vec![House {
            id: "h".to_owned(),
            name: "The House of Test".to_owned(),
            seat_region_id: region_id,
            founder_name: "Test".to_owned(),
            member_ids: vec!["scion".to_owned()],
            prestige: 10.0,
        }];

        let tick = |world: &mut WorldState| {
            tick_houses(
                &mut world.houses,
                &world.heroes,
                &world.regions,
                &balance,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            )
        };

        tick(&mut world);
        assert_eq!(
            world.houses[0].prestige, 150.0,
            "prestige tracks the living line's renown"
        );

        // The line dies out: prestige drifts to nothing and the house is forgotten.
        world.heroes[0].is_alive = false;
        for _ in 0..50 {
            if world.houses.is_empty() {
                break;
            }
            tick(&mut world);
        }
        assert!(
            world.houses.is_empty(),
            "a house with no living blood and no standing is forgotten"
        );
    }

    #[test]
    fn a_house_whose_seat_is_lost_follows_its_blood() {
        // When a house's seat region vanishes (conquered away), the house
        // reestablishes itself where its greatest living scion dwells (GDD 5.4
        // <-> 5.2). A house with no living blood keeps its lost seat and fades.
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let refuge = world.regions[1].id.clone();

        // A scion of a house whose seat, "lost-realm", is not on the map. The
        // scion has fled to a surviving region.
        world.heroes = vec![legend("scion", &refuge, 120.0)];
        world.houses = vec![House {
            id: "h".to_owned(),
            name: "The House of Exile".to_owned(),
            seat_region_id: "lost-realm".to_owned(),
            founder_name: "Exile".to_owned(),
            member_ids: vec!["scion".to_owned()],
            prestige: 120.0,
        }];

        tick_houses(
            &mut world.houses,
            &world.heroes,
            &world.regions,
            &data.balance.house,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );

        assert_eq!(
            world.houses[0].seat_region_id, refuge,
            "a displaced house reseats where its blood dwells"
        );
        assert!(
            world
                .chronicle
                .iter_newest()
                .any(|e| e.message.contains("reestablishes")),
            "the reseating is chronicled"
        );
    }

    #[test]
    fn an_heir_born_on_a_houses_seat_inherits_its_renown() {
        // A descendant born in a house's seat region joins its line and inherits a
        // share of its prestige; one born elsewhere is baseborn.
        let data = GameData::load().unwrap();
        let balance = &data.balance.house;
        let mut world = WorldState::new(&data);
        let seat = world.regions[0].id.clone();
        let elsewhere = world.regions[1].id.clone();
        world.houses = vec![House {
            id: "h".to_owned(),
            name: "The House of Test".to_owned(),
            seat_region_id: seat.clone(),
            founder_name: "Test".to_owned(),
            member_ids: vec!["founder".to_owned()],
            prestige: 100.0,
        }];

        let far = maybe_inherit(
            &mut world.houses,
            "far",
            &elsewhere,
            balance,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        assert_eq!(far, 0.0, "a child born off the seat inherits nothing");

        let heir = maybe_inherit(
            &mut world.houses,
            "heir-1",
            &seat,
            balance,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        assert_eq!(
            heir,
            100.0 * balance.inherit_fraction,
            "an heir born on the seat inherits a share of the prestige"
        );
        assert!(
            world.houses[0].holds("heir-1"),
            "the heir joins its house's line"
        );
    }

    #[test]
    fn with_no_houses_an_heir_is_baseborn() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        world.houses.clear();
        let renown = maybe_inherit(
            &mut world.houses,
            "heir",
            "aldermoor",
            &data.balance.house,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        assert_eq!(renown, 0.0, "with no houses, a newborn inherits nothing");
    }
}
