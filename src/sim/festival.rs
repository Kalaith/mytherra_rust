//! Per-tick festivals (GDD 5.2 <-> 6): the world's great celebrations, the
//! constructive mirror of the crisis systems. Once in a generation — on a fixed
//! cadence — the world's foremost realm, flourishing and at peace, throws open its
//! gates for a festival the age remembers. While it lasts it draws the world's eye:
//! deepening the host's cultural renown and its faith, and crowning the heroes who
//! dwell there with the honour of the games and rites, so a golden land's fortune
//! feeds its culture, its faith, and its legends all at once. Then it passes into
//! memory. Deterministic: kindling runs on the calendar, not a roll, and its boons
//! are read straight from balance — the seeded RNG stream is untouched, and the
//! boons deliberately never touch the crisis levers a runaway could feed on.

use crate::data::strings::{ChronicleText, FestivalNames};
use crate::data::{fill, FestivalBalance};
use crate::world::{Chronicle, EventKind, Festival, Hero, Region};

#[allow(clippy::too_many_arguments)]
pub fn tick_festivals(
    festivals: &mut Vec<Festival>,
    regions: &mut [Region],
    heroes: &mut [Hero],
    seq: &mut u64,
    balance: &FestivalBalance,
    names: &FestivalNames,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    // Advance every standing festival: while it runs it lifts its host and honours
    // the heroes who dwell there, then — its years spent — passes into memory. A
    // festival whose host has since been conquered away or sundered simply counts
    // down unremembered, its boons falling on no land.
    festivals.retain_mut(|festival| {
        let host_name = regions
            .iter()
            .find(|r| r.id == festival.region_id)
            .map(|r| r.name.clone());
        if let Some(region) = regions.iter_mut().find(|r| r.id == festival.region_id) {
            region.add_cultural_influence(balance.culture_boon);
            region.add_resonance(balance.resonance_boon);
        }
        for hero in heroes.iter_mut() {
            if hero.is_alive && hero.region_id == festival.region_id {
                hero.renown += balance.renown_boon;
            }
        }

        festival.remaining -= 1;
        if festival.remaining == 0 {
            if let Some(host_name) = host_name {
                chronicle.push(
                    year,
                    EventKind::Region,
                    fill(
                        &text.festival_ends,
                        &[("festival", festival.name.clone()), ("region", host_name)],
                    ),
                );
            }
            false
        } else {
            true
        }
    });

    // Kindle a new festival on the generational cadence, in the world's single
    // foremost eligible realm — the most culturally prominent land that is also
    // rich enough to bear the cost and calm enough to celebrate — but only when no
    // festival already stands, so the world holds one great celebration at a time.
    if balance.interval == 0 || !year.is_multiple_of(balance.interval) || !festivals.is_empty() {
        return;
    }
    let host = regions
        .iter()
        .filter(|r| {
            r.prosperity >= balance.min_prosperity
                && r.cultural_influence >= balance.min_culture
                && r.chaos <= balance.max_chaos
        })
        .max_by(|a, b| {
            a.cultural_influence
                .total_cmp(&b.cultural_influence)
                .then_with(|| a.id.cmp(&b.id))
        });
    if let Some(host) = host {
        *seq += 1;
        let name = names.pick(*seq).to_owned();
        chronicle.push(
            year,
            EventKind::Region,
            fill(
                &text.festival_begins,
                &[("festival", name.clone()), ("region", host.name.clone())],
            ),
        );
        festivals.push(Festival {
            id: format!("festival-{seq}"),
            name,
            region_id: host.id.clone(),
            remaining: balance.duration,
            began_year: year,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::world::WorldState;

    fn run(world: &mut WorldState, data: &GameData, year: u32) {
        world.year = year;
        tick_festivals(
            &mut world.festivals,
            &mut world.regions,
            &mut world.heroes,
            &mut world.festival_seq,
            &data.balance.festival,
            &data.strings.festivals,
            &mut world.chronicle,
            &data.strings.chronicle,
            year,
        );
    }

    #[test]
    fn the_foremost_flourishing_realm_holds_a_festival_on_the_cadence() {
        let data = GameData::load().unwrap();
        let b = &data.balance.festival;
        let mut world = WorldState::new(&data);
        world.regions.truncate(2);
        // Region 0 is the world's cultural heart; region 1 is prominent but less so.
        for (i, r) in world.regions.iter_mut().enumerate() {
            r.prosperity = b.min_prosperity + 10.0;
            r.chaos = b.max_chaos - 5.0;
            r.cultural_influence = b.min_culture + if i == 0 { 20.0 } else { 5.0 };
        }
        let heart = world.regions[0].id.clone();

        // Off the cadence, no festival is raised.
        run(&mut world, &data, b.interval + 1);
        assert!(
            world.festivals.is_empty(),
            "no festival between the reckonings"
        );

        // On the cadence, the foremost realm holds one.
        run(&mut world, &data, b.interval);
        assert_eq!(world.festivals.len(), 1, "the cadence raises a festival");
        assert_eq!(
            world.festivals[0].region_id, heart,
            "the most culturally prominent eligible realm hosts"
        );
    }

    #[test]
    fn a_strife_torn_or_poor_world_holds_no_festival() {
        let data = GameData::load().unwrap();
        let b = &data.balance.festival;
        let mut world = WorldState::new(&data);
        // Prominent and rich, but wracked by chaos: no celebration.
        for r in world.regions.iter_mut() {
            r.prosperity = b.min_prosperity + 10.0;
            r.cultural_influence = b.min_culture + 10.0;
            r.chaos = b.max_chaos + 20.0;
        }
        run(&mut world, &data, b.interval);
        assert!(
            world.festivals.is_empty(),
            "a strife-torn world throws no festival however rich"
        );
    }

    #[test]
    fn a_festival_lifts_its_host_and_crowns_its_heroes_then_passes() {
        use crate::data::{HeroRole, HeroSeed};
        let data = GameData::load().unwrap();
        let b = &data.balance.festival;
        let mut world = WorldState::new(&data);
        world.regions.truncate(1);
        let host_id = world.regions[0].id.clone();
        world.regions[0].prosperity = b.min_prosperity + 10.0;
        world.regions[0].chaos = b.max_chaos - 5.0;
        world.regions[0].cultural_influence = b.min_culture + 10.0;
        world.regions[0].divine_resonance = 40.0;
        world.heroes = vec![Hero::from_seed(&HeroSeed {
            id: "reveler".to_owned(),
            name: "Reveler".to_owned(),
            role: HeroRole::Warrior,
            region_id: host_id.clone(),
            level: 3,
            age: 30,
        })];

        let culture_before = world.regions[0].cultural_influence;
        let resonance_before = world.regions[0].divine_resonance;
        let renown_before = world.heroes[0].renown;

        // Begin the festival, then run it out its full duration.
        run(&mut world, &data, b.interval);
        assert_eq!(world.festivals.len(), 1);
        for y in 1..=b.duration {
            run(&mut world, &data, b.interval + y);
        }

        assert!(
            world.festivals.is_empty(),
            "a festival passes into memory once its years are spent"
        );
        assert!(
            world.regions[0].cultural_influence > culture_before,
            "a festival deepens its host's cultural renown"
        );
        assert!(
            world.regions[0].divine_resonance > resonance_before,
            "a festival deepens its host's faith"
        );
        assert!(
            world.heroes[0].renown > renown_before,
            "a festival crowns the heroes who dwell in its host"
        );
    }
}
