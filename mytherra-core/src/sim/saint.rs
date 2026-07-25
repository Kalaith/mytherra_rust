//! Per-tick saints (GDD 5.1 <-> 5.4): the veneration of the great dead. When one
//! of the holy — a Cleric of high renown — or one of the truly legendary passes,
//! the faithful of their home land raise them to sainthood, and the remembered
//! example hallows that land's faith for as long as the memory endures. A saint's
//! veneration begins fierce at canonization and fades over the ages until the soul
//! passes from living memory. The faith legacy to set beside the House's bloodline
//! and the Order's calling. Deterministic: the dead are scanned, no roll decides a
//! saint.

use crate::data::strings::ChronicleText;
use crate::data::{fill, HeroRole, SaintBalance};
use crate::world::{Chronicle, EventKind, Hero, Region, Saint};

/// A soul raised to sainthood this tick: `(saint_name, region_id, region_name)`,
/// returned so the caller can commemorate it in a saint's myth (GDD 5.1 <-> 5.6).
pub type NewSaint = (String, String, String);

#[allow(clippy::too_many_arguments)]
pub fn tick_saints(
    saints: &mut Vec<Saint>,
    heroes: &[Hero],
    regions: &mut [Region],
    seq: &mut u64,
    balance: &SaintBalance,
    legend_bar: f32,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) -> Vec<NewSaint> {
    // Souls raised this tick, so the caller may commemorate each in a saint's myth
    // (GDD 5.1 <-> 5.6) — the faith counterpart to a slain beast's Valor tale.
    let mut canonized: Vec<NewSaint> = Vec::new();

    // The dead worthy of sainthood: a dead Cleric past the renown floor (venerated
    // for holiness), or any dead hero who reached the legend bar besides (for sheer
    // greatness) — not sainted already, and with a homeland still standing to keep
    // their memory. Gathered rather than raised in place so a death-wave doesn't
    // canonize a whole cohort in one year: the worthiest go first (most renown, id
    // as the deterministic tie-break), and only up to the year's cadence, the rest
    // left eligible for the years that follow.
    let mut worthy: Vec<&Hero> = heroes
        .iter()
        .filter(|hero| {
            !hero.is_alive
                && hero.renown >= balance.renown_threshold
                && (hero.role == HeroRole::Cleric || hero.renown >= legend_bar)
                && !saints.iter().any(|s| s.hero_id == hero.id)
                && regions.iter().any(|r| r.id == hero.region_id)
        })
        .collect();
    worthy.sort_by(|a, b| b.renown.total_cmp(&a.renown).then_with(|| a.id.cmp(&b.id)));

    for hero in worthy
        .into_iter()
        .take(balance.canonizations_per_tick as usize)
    {
        let region_name = regions
            .iter()
            .find(|r| r.id == hero.region_id)
            .map(|r| r.name.clone())
            .expect("a worthy soul's homeland was confirmed to still stand");
        *seq += 1;
        let name = fill(&text.saint_name, &[("hero", hero.name.clone())]);
        saints.push(Saint {
            id: format!("saint-{seq}"),
            name: name.clone(),
            hero_id: hero.id.clone(),
            region_id: hero.region_id.clone(),
            veneration: balance.start_veneration,
            canonized_year: year,
        });
        chronicle.push(
            year,
            EventKind::Hero,
            fill(
                &text.saint_canonized,
                &[("saint", name.clone()), ("region", region_name.clone())],
            ),
        );
        canonized.push((name, hero.region_id.clone(), region_name));
    }

    // Memory fades: each saint's veneration ebbs a little each tick, and one worn
    // below the floor has passed from living memory and is forgotten.
    saints.retain_mut(|saint| {
        saint.veneration -= balance.veneration_decay;
        if saint.veneration < balance.forgotten_floor {
            chronicle.push(
                year,
                EventKind::Region,
                fill(&text.saint_forgotten, &[("saint", saint.name.clone())]),
            );
            return false;
        }
        true
    });

    // A region's patron — its single most-venerated saint — hallows it, raising
    // its divine resonance in measure of the devotion still owed. Only the patron
    // counts: a land reveres its greatest saint, it does not simply pile the
    // devotion owed every soul it has ever buried atop one another, so a realm of
    // many saints is no more hallowed than one with a single towering patron.
    for region in regions.iter_mut() {
        let patron = saints
            .iter()
            .filter(|s| s.region_id == region.id)
            .map(|s| s.veneration)
            .fold(0.0_f32, f32::max);
        if patron > 0.0 {
            region.add_resonance(patron * balance.resonance_per_veneration);
        }
    }

    canonized
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::world::WorldState;

    fn run(world: &mut WorldState, data: &GameData, legend_bar: f32) {
        tick_saints(
            &mut world.saints,
            &world.heroes,
            &mut world.regions,
            &mut world.saint_seq,
            &data.balance.saint,
            legend_bar,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
    }

    fn dead_hero(id: &str, role: HeroRole, region: &str, renown: f32) -> Hero {
        Hero {
            id: id.to_owned(),
            name: format!("Hero {id}"),
            role,
            region_id: region.to_owned(),
            level: 8,
            age: 80,
            is_alive: false,
            renown,
        }
    }

    #[test]
    fn the_holy_and_the_legendary_dead_are_raised_to_sainthood() {
        let data = GameData::load().unwrap();
        let b = &data.balance.saint;
        let legend_bar = 180.0;
        let mut world = WorldState::new(&data);
        let region = world.regions[0].id.clone();

        world.heroes = vec![
            // A dead Cleric past the renown floor — canonized for holiness.
            dead_hero(
                "cleric",
                HeroRole::Cleric,
                &region,
                b.renown_threshold + 5.0,
            ),
            // A dead Warrior past the floor but short of legend — NOT canonized.
            dead_hero(
                "warrior",
                HeroRole::Warrior,
                &region,
                b.renown_threshold + 5.0,
            ),
            // A dead Warrior who reached legend — canonized for sheer greatness.
            dead_hero("legend", HeroRole::Warrior, &region, legend_bar + 10.0),
            // A living Cleric of great renown — the living are not sainted.
            {
                let mut h = dead_hero("living", HeroRole::Cleric, &region, legend_bar + 50.0);
                h.is_alive = true;
                h
            },
        ];

        // Canonization is throttled to the year's cadence (1/tick): the worthiest
        // eligible soul is raised first — here the legend, at renown 190, ahead of
        // the cleric at 105 — and the rest follow in later years.
        run(&mut world, &data, legend_bar);
        let after_one: Vec<&str> = world.saints.iter().map(|s| s.hero_id.as_str()).collect();
        assert_eq!(
            after_one,
            vec!["legend"],
            "the most renowned dead is sainted first, one per year"
        );

        // The next year raises the cleric; the merely-renowned warrior and the
        // living are never raised.
        run(&mut world, &data, legend_bar);
        let sainted: Vec<&str> = world.saints.iter().map(|s| s.hero_id.as_str()).collect();
        assert!(sainted.contains(&"cleric"), "a holy dead Cleric is sainted");
        assert!(
            sainted.contains(&"legend"),
            "a legendary dead hero is sainted"
        );
        assert!(
            !sainted.contains(&"warrior"),
            "a merely-renowned non-Cleric is not sainted"
        );
        assert!(!sainted.contains(&"living"), "the living are not sainted");

        // Canonized once only, however many ticks pass.
        run(&mut world, &data, legend_bar);
        assert_eq!(
            world
                .saints
                .iter()
                .filter(|s| s.hero_id == "cleric")
                .count(),
            1,
            "a soul is never sainted twice"
        );
    }

    #[test]
    fn a_saint_hallows_its_land_then_fades_from_memory() {
        let data = GameData::load().unwrap();
        let b = &data.balance.saint;
        let mut world = WorldState::new(&data);
        let region = world.regions[0].id.clone();
        world.regions[0].divine_resonance = 50.0;
        world.heroes = vec![dead_hero(
            "cleric",
            HeroRole::Cleric,
            &region,
            b.renown_threshold + 5.0,
        )];

        run(&mut world, &data, 180.0);
        assert_eq!(world.saints.len(), 1);
        assert!(
            world.regions[0].divine_resonance > 50.0,
            "a fresh saint hallows its home region's faith"
        );

        // Left to the ages, the veneration fades and the saint is forgotten.
        let mut forgotten = false;
        for _ in 0..1000 {
            run(&mut world, &data, 180.0);
            if world.saints.is_empty() {
                forgotten = true;
                break;
            }
        }
        assert!(forgotten, "a saint's memory should fade in the end");
    }
}
