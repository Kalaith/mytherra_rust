//! Per-tick hero lifecycle: level-up, aging, death, and region movement
//! (GDD 5.4). All randomness flows through the world-owned `SeededRng` so the
//! sim stays deterministic and auditable.

use crate::data::strings::ChronicleText;
use crate::data::{fill, HeroBalance, HeroRole, MigrationBalance, RegionBalance};
use crate::sim::culture::hero_culture;
use crate::world::{Chronicle, EventKind, Hero, Landmark, Plague, Region, Settlement};
use macroquad_toolkit::rng::SeededRng;

/// Advance every living hero by one world tick.
#[allow(clippy::too_many_arguments)]
pub fn tick_heroes(
    heroes: &mut [Hero],
    regions: &[Region],
    landmarks: &[Landmark],
    settlements: &[Settlement],
    tier_thresholds: &[f32],
    rng: &mut SeededRng,
    balance: &HeroBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    // Each region's fame — the renown of its most famed living hero — snapshotted
    // before the roster moves, so a land home to a legend or champion draws the
    // ambitious this tick (GDD 5.4). Read now, since the loop below both mutates
    // heroes and reads their homes.
    let region_fame = region_fame(heroes, regions);

    for hero in heroes.iter_mut() {
        if !hero.is_alive {
            continue;
        }

        // Trial by fire: a hero grows faster in a dangerous land than a placid
        // one (GDD 5.4), so peril tempers those who dwell in it — and faster still
        // in a land whose character suits their calling (a warrior in a martial
        // land), so a region's culture shapes the heroes who rise in it.
        let home = regions.iter().find(|r| r.id == hero.region_id);
        let danger = home.map(|r| r.danger).unwrap_or(0.0);
        let culture_match = home.is_some_and(|r| r.culture == hero_culture(hero.role));
        if rng.chance(hero.level_up_chance_in(danger, culture_match, balance)) {
            hero.level += 1;
            hero.renown += balance.renown.per_level;
            // Chronicle only milestone levels, so a hero's steady climb marks the
            // Event Log at intervals rather than on every step (GDD 10).
            if hero.level % balance.level_up.chronicle_interval.max(1) == 0 {
                chronicle.push(
                    year,
                    EventKind::Hero,
                    fill(
                        &text.hero_level_up,
                        &[
                            ("hero", hero.name.clone()),
                            ("region", region_name(regions, &hero.region_id)),
                            ("level", hero.level.to_string()),
                        ],
                    ),
                );
            }
        }

        hero.age += 1;

        if rolls_death(hero, regions, rng, balance) {
            hero.is_alive = false;
            let legend_bar = balance
                .renown
                .thresholds
                .last()
                .copied()
                .unwrap_or(f32::INFINITY);
            chronicle.push(
                year,
                EventKind::Hero,
                fill(
                    death_line(hero.renown, legend_bar, text),
                    &[
                        ("hero", hero.name.clone()),
                        ("region", region_name(regions, &hero.region_id)),
                    ],
                ),
            );
            continue;
        }

        if rng.chance(balance.move_chance) {
            if let Some(dest) = pick_destination(
                regions,
                landmarks,
                settlements,
                tier_thresholds,
                &region_fame,
                &hero.region_id,
                hero.role,
                rng,
                &balance.migration,
            ) {
                hero.region_id = dest;
            }
        }
    }
}

/// Each region's fame — the greatest renown among its living resident heroes —
/// aligned to `regions` by index (GDD 5.4). A region with a champion or a living
/// legend has high fame; one of unknowns has none.
fn region_fame(heroes: &[Hero], regions: &[Region]) -> Vec<f32> {
    regions
        .iter()
        .map(|r| {
            heroes
                .iter()
                .filter(|h| h.is_alive && h.region_id == r.id)
                .map(|h| h.renown)
                .fold(0.0_f32, f32::max)
        })
        .collect()
}

/// A land tends and turns to its faith (GDD 5.4 <-> 5.1). Two forces raise a
/// region's divine resonance each tick: the resident Clerics who tend it — the
/// passive, favor-free counterpart to the player's consecration, and the Cleric
/// role's own domain — and affliction itself, for a land gripped by famine or
/// pestilence crowds its temples as the desperate beg deliverance. So faith grows
/// both where the devout dwell and where the world's scourges fall, and a
/// comfortable land forgets the gods a suffering one turns to. Deterministic: no
/// RNG.
pub fn tick_faith(
    heroes: &[Hero],
    regions: &mut [Region],
    plagues: &[Plague],
    balance: &HeroBalance,
) {
    for region in regions.iter_mut() {
        let clerics = heroes
            .iter()
            .filter(|h| h.is_alive && h.role == HeroRole::Cleric && h.region_id == region.id)
            .count();
        let mut gain = clerics as f32 * balance.cleric_resonance_per_tick;

        // Catastrophe drives the desperate to prayer: a famine-struck or
        // plague-ridden land turns to the gods, its faith surging while the
        // affliction lasts.
        let afflicted = region.famine || plagues.iter().any(|p| p.region_id == region.id);
        if afflicted {
            gain += balance.affliction_resonance_per_tick;
        }

        if gain != 0.0 {
            region.add_resonance(gain);
        }
    }
}

/// A land's resident Warriors garrison it: their presence lowers their home
/// region's danger a little every tick (GDD 5.4 <-> 5.2), scaled by their levels,
/// so a land defended by seasoned fighters grows safer over time. This is the
/// passive, day-to-day counterpart to the conquest might those same warriors lend
/// when a border war comes (`resident_might`) — the Warrior role's per-tick domain
/// beside the Cleric's faith and the Merchant's trade. Deterministic: no RNG.
pub fn tick_garrison(
    heroes: &[Hero],
    regions: &mut [Region],
    balance: &HeroBalance,
    region_balance: &RegionBalance,
) {
    if balance.warrior_danger_relief <= 0.0 {
        return;
    }
    for region in regions.iter_mut() {
        let garrison: u32 = heroes
            .iter()
            .filter(|h| h.is_alive && h.role == HeroRole::Warrior && h.region_id == region.id)
            .map(|h| h.level)
            .sum();
        if garrison > 0 {
            region.apply_deltas(
                0.0,
                0.0,
                -balance.warrior_danger_relief * garrison as f32,
                0.0,
                region_balance,
            );
        }
    }
}

/// Death roll for one hero: elders past their life expectancy roll a flat
/// chance; younger heroes face a danger-scaled, level-mitigated chance.
fn rolls_death(
    hero: &Hero,
    regions: &[Region],
    rng: &mut SeededRng,
    balance: &HeroBalance,
) -> bool {
    let death = &balance.death;
    if hero.age as f32 > hero.life_expectancy(balance) {
        return rng.chance(death.elder_roll);
    }
    let danger = region_danger(regions, &hero.region_id);
    rng.chance(danger_death_chance(hero, danger, balance))
}

/// Which death line a fallen hero earns: one who had already crossed into legend
/// (top renown title) gets the commemorative variant, everyone else the plain one.
fn death_line(renown: f32, legend_bar: f32, text: &ChronicleText) -> &str {
    if renown >= legend_bar {
        &text.hero_legend_death
    } else {
        &text.hero_death
    }
}

/// A young hero's per-tick chance of a violent death. Level and hard-won renown
/// both stave it off — a legend clings to life against the odds — but never
/// below the floor.
fn danger_death_chance(hero: &Hero, danger: f32, balance: &HeroBalance) -> f32 {
    let death = &balance.death;
    (danger / death.danger_divisor
        - hero.level as f32 / death.level_divisor
        - hero.renown * balance.renown.survival_coeff)
        .max(death.min_chance)
}

fn region_danger(regions: &[Region], region_id: &str) -> f32 {
    regions
        .iter()
        .find(|r| r.id == region_id)
        .map(|r| r.danger)
        .unwrap_or(0.0)
}

fn region_name(regions: &[Region], region_id: &str) -> String {
    regions
        .iter()
        .find(|r| r.id == region_id)
        .map(|r| r.name.clone())
        .unwrap_or_else(|| region_id.to_owned())
}

/// How strongly a region draws a hero of the given role (GDD 5.4). Each role
/// weights the region's stats differently, floored so the pull is always
/// positive. This is what makes warriors flow toward danger and scholars toward
/// settled, cultured lands — wonders of the hero's own culture add their own pull
/// (so great works draw the kind of people who raise them, GDD 5.2), and the tier
/// of the region's greatest city lures every role toward the great cities where
/// fame and fortune gather (GDD 5.3). `city_tier` is that greatest tier.
fn attractiveness(
    region: &Region,
    landmarks: &[Landmark],
    city_tier: f32,
    role: HeroRole,
    mig: &MigrationBalance,
) -> f32 {
    let w = mig.roles.get(role);
    let kin_culture = hero_culture(role);
    let kin_wonders = landmarks
        .iter()
        .filter(|l| l.region_id == region.id && l.culture == kin_culture)
        .count() as f32;
    (mig.base_weight
        + w.prosperity * region.prosperity
        + w.danger * region.danger
        + w.magic * region.magic_affinity
        + w.culture * region.cultural_influence
        + w.resonance * region.divine_resonance
        + mig.wonder_pull * kin_wonders
        + mig.city_pull * city_tier)
        .max(mig.min_weight)
}

/// The size tier of a region's greatest city (0 if it holds no settlements), the
/// lure that draws heroes toward its great cities.
fn greatest_city_tier(region_id: &str, settlements: &[Settlement], tier_thresholds: &[f32]) -> f32 {
    settlements
        .iter()
        .filter(|s| s.region_id == region_id)
        .map(|s| s.tier(tier_thresholds))
        .max()
        .unwrap_or(0) as f32
}

/// Pick a destination region other than the hero's current one, weighted by how
/// attractive each is to the hero's role. Deterministic given the RNG state: a
/// single roll walks the cumulative weight.
#[allow(clippy::too_many_arguments)]
fn pick_destination(
    regions: &[Region],
    landmarks: &[Landmark],
    settlements: &[Settlement],
    tier_thresholds: &[f32],
    region_fame: &[f32],
    current: &str,
    role: HeroRole,
    rng: &mut SeededRng,
    mig: &MigrationBalance,
) -> Option<String> {
    let candidates: Vec<(&str, f32)> = regions
        .iter()
        .enumerate()
        .filter(|(_, r)| r.id != current)
        .map(|(i, r)| {
            let city_tier = greatest_city_tier(&r.id, settlements, tier_thresholds);
            // The pull of the land's own state and works, plus the beacon of its
            // most famed resident — heroes flock to where legends dwell (GDD 5.4).
            let fame = region_fame.get(i).copied().unwrap_or(0.0);
            let weight = attractiveness(r, landmarks, city_tier, role, mig) + mig.fame_pull * fame;
            (r.id.as_str(), weight)
        })
        .collect();
    if candidates.is_empty() {
        return None;
    }
    let total: f32 = candidates.iter().map(|(_, w)| *w).sum();
    let mut roll = rng.next_f32() * total;
    for (id, weight) in &candidates {
        roll -= *weight;
        if roll <= 0.0 {
            return Some((*id).to_owned());
        }
    }
    // Floating-point fallthrough: take the last candidate.
    Some(candidates[candidates.len() - 1].0.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{ClimateType, Culture, GameData, HeroSeed, LandmarkSeed, RegionSeed};
    use crate::world::WorldState;

    #[test]
    fn a_legend_earns_a_commemorative_death_line() {
        let data = GameData::load().unwrap();
        let text = &data.strings.chronicle;
        let bar = *data.balance.hero.renown.thresholds.last().unwrap();
        assert_eq!(death_line(bar + 1.0, bar, text), text.hero_legend_death);
        assert_eq!(death_line(bar - 1.0, bar, text), text.hero_death);
    }

    #[test]
    fn a_living_cleric_tends_the_faith_of_its_home_region() {
        let data = GameData::load().unwrap();
        let balance = &data.balance.hero;
        // Two regions at the neutral resonance baseline (50): one home to clerics,
        // one barren of them.
        let mut regions = vec![
            region("home", 60.0, 20.0, 40.0, 40.0),
            region("barren", 60.0, 20.0, 40.0, 40.0),
        ];

        // At home: one living cleric (counts), a warrior (wrong role), and a
        // fallen cleric (dead). Only the living cleric should tend the faith.
        let warrior = hero("fighter", HeroRole::Warrior, "home");
        let mut fallen = hero("martyr", HeroRole::Cleric, "home");
        fallen.is_alive = false;
        let cleric = hero("holy", HeroRole::Cleric, "home");

        tick_faith(&[cleric, warrior, fallen], &mut regions, &[], balance);

        assert!(
            (regions[0].divine_resonance - (50.0 + balance.cleric_resonance_per_tick)).abs() < 1e-4,
            "exactly one living cleric should raise home resonance by one step"
        );
        assert_eq!(
            regions[1].divine_resonance, 50.0,
            "a land with no clerics keeps its faith unchanged"
        );
    }

    #[test]
    fn affliction_drives_the_people_to_prayer() {
        use crate::world::Plague;
        let data = GameData::load().unwrap();
        let balance = &data.balance.hero;
        // Three cleric-less regions at the resonance baseline: one calm, one in
        // famine, one gripped by plague. Only the afflicted turn to the gods.
        let mut regions = vec![
            region("calm", 60.0, 20.0, 40.0, 40.0),
            region("starving", 60.0, 20.0, 40.0, 40.0),
            region("plagued", 60.0, 20.0, 40.0, 40.0),
        ];
        regions[1].famine = true;
        let plagues = vec![Plague {
            id: "p".to_owned(),
            name: "The Test Fever".to_owned(),
            region_id: "plagued".to_owned(),
            severity: 1.0,
            age: 0,
        }];

        tick_faith(&[], &mut regions, &plagues, balance);

        assert_eq!(
            regions[0].divine_resonance, 50.0,
            "a calm, unafflicted land's faith holds steady"
        );
        let expected = 50.0 + balance.affliction_resonance_per_tick;
        assert!(
            (regions[1].divine_resonance - expected).abs() < 1e-4,
            "a starving land turns to prayer"
        );
        assert!(
            (regions[2].divine_resonance - expected).abs() < 1e-4,
            "a plague-ridden land turns to prayer"
        );
    }

    #[test]
    fn resident_warriors_garrison_their_region_and_lower_its_danger() {
        let data = GameData::load().unwrap();
        let balance = &data.balance.hero;
        // Two regions at equal danger: one garrisoned, one open.
        let mut regions = vec![
            region("held", 60.0, 40.0, 40.0, 40.0),
            region("open", 60.0, 40.0, 40.0, 40.0),
        ];

        // At the held region: a living warrior (garrisons), a cleric (wrong role),
        // and a fallen warrior (dead). Only the living warrior lowers danger.
        let warrior = hero("guard", HeroRole::Warrior, "held"); // level 5
        let cleric = hero("holy", HeroRole::Cleric, "held");
        let mut fallen = hero("martyr", HeroRole::Warrior, "held");
        fallen.is_alive = false;

        tick_garrison(
            &[warrior, cleric, fallen],
            &mut regions,
            balance,
            &data.balance.region,
        );

        // Relief is exactly the living warrior's levels times the coefficient.
        let expected = 40.0 - balance.warrior_danger_relief * 5.0;
        assert!(
            (regions[0].danger - expected).abs() < 1e-4,
            "a garrisoned land should grow safer by its warriors' levels"
        );
        assert_eq!(
            regions[1].danger, 40.0,
            "an ungarrisoned land keeps its peril"
        );
    }

    #[test]
    fn heroes_flock_to_where_legends_dwell() {
        // Two identical regions; the one home to a famed hero draws migrating
        // heroes more often than the fameless one (GDD 5.4).
        let data = GameData::load().unwrap();
        let mig = &data.balance.hero.migration;
        let regions = vec![
            region("plain", 50.0, 20.0, 20.0, 20.0),
            region("storied", 50.0, 20.0, 20.0, 20.0),
        ];
        // The storied land is home to a living legend; the plain land to unknowns.
        let fame = [0.0, 300.0];
        let mut rng = SeededRng::new(11);
        let mut to_storied = 0;
        for _ in 0..1000 {
            // The mover hails from a third region, so both are candidates.
            if let Some(dest) = pick_destination(
                &regions,
                &[],
                &[],
                &[],
                &fame,
                "elsewhere",
                HeroRole::Warrior,
                &mut rng,
                mig,
            ) {
                if dest == "storied" {
                    to_storied += 1;
                }
            }
        }
        assert!(
            to_storied > 550,
            "heroes should favour the storied land ({to_storied}/1000)"
        );
    }

    fn region(id: &str, prosperity: f32, danger: f32, magic: f32, culture: f32) -> Region {
        let balance = GameData::load().unwrap().balance.region;
        Region::from_seed(
            &RegionSeed {
                id: id.to_owned(),
                name: id.to_owned(),
                climate: ClimateType::Temperate,
                culture: Culture::Martial,
                prosperity,
                chaos: 30.0,
                danger,
                magic_affinity: magic,
                population: 5000.0,
                cultural_influence: culture,
                divine_resonance: 50.0,
            },
            &balance,
        )
    }

    fn hero(id: &str, role: HeroRole, region_id: &str) -> Hero {
        Hero::from_seed(&HeroSeed {
            id: id.to_owned(),
            name: id.to_owned(),
            role,
            region_id: region_id.to_owned(),
            level: 5,
            age: 30,
        })
    }

    #[test]
    fn migration_weights_pull_each_role_differently() {
        let data = GameData::load().unwrap();
        let mig = &data.balance.hero.migration;
        let dangerous = region("war", 25.0, 90.0, 20.0, 20.0);
        let settled = region("haven", 90.0, 10.0, 30.0, 85.0);

        // A warrior is drawn to conflict; a scholar toward settled, cultured land.
        assert!(
            attractiveness(&dangerous, &[], 0.0, HeroRole::Warrior, mig)
                > attractiveness(&settled, &[], 0.0, HeroRole::Warrior, mig)
        );
        assert!(
            attractiveness(&settled, &[], 0.0, HeroRole::Scholar, mig)
                > attractiveness(&dangerous, &[], 0.0, HeroRole::Scholar, mig)
        );
        // A mage follows magic.
        let arcane = region("spire", 50.0, 30.0, 95.0, 40.0);
        assert!(
            attractiveness(&arcane, &[], 0.0, HeroRole::Mage, mig)
                > attractiveness(&settled, &[], 0.0, HeroRole::Mage, mig)
        );
    }

    #[test]
    fn a_cleric_makes_pilgrimage_to_hallowed_ground() {
        let data = GameData::load().unwrap();
        let mig = &data.balance.hero.migration;
        // Two lands alike but for their faith.
        let mut hallowed = region("shrine", 60.0, 20.0, 40.0, 40.0);
        let mut faithless = region("waste", 60.0, 20.0, 40.0, 40.0);
        hallowed.divine_resonance = 95.0;
        faithless.divine_resonance = 20.0;

        // A cleric is drawn to the hallowed land above the faithless one.
        assert!(
            attractiveness(&hallowed, &[], 0.0, HeroRole::Cleric, mig)
                > attractiveness(&faithless, &[], 0.0, HeroRole::Cleric, mig),
            "a cleric should make pilgrimage toward hallowed ground"
        );
        // A warrior answers no such call, so resonance does not sway them.
        assert_eq!(
            attractiveness(&hallowed, &[], 0.0, HeroRole::Warrior, mig),
            attractiveness(&faithless, &[], 0.0, HeroRole::Warrior, mig),
            "divine resonance should not move a hero who does not answer its call"
        );
    }

    #[test]
    fn the_lure_of_a_great_city_draws_every_role() {
        let data = GameData::load().unwrap();
        let mig = &data.balance.hero.migration;
        let land = region("aldervale", 60.0, 20.0, 40.0, 60.0);
        // The same land pulls harder when it holds a great city than none, for any
        // role — heroes seek the fame and fortune of the metropolis.
        for role in [HeroRole::Warrior, HeroRole::Scholar, HeroRole::Mage] {
            assert!(
                attractiveness(&land, &[], 4.0, role, mig)
                    > attractiveness(&land, &[], 0.0, role, mig),
                "a great city should draw heroes of every calling"
            );
        }

        // greatest_city_tier reports the flagship city's tier among the towns.
        let thresholds = &data.balance.settlement.tier_thresholds;
        let town = |id: &str, pop: f32| Settlement {
            id: id.to_owned(),
            name: "T".to_owned(),
            region_id: "aldervale".to_owned(),
            population: pop,
            prosperity: 50.0,
        };
        let towns = vec![town("a", 800.0), town("b", 40_000.0), town("c", 3_000.0)];
        assert_eq!(
            greatest_city_tier("aldervale", &towns, thresholds),
            town("b", 40_000.0).tier(thresholds) as f32,
            "the greatest city's tier is what draws heroes"
        );
        assert_eq!(
            greatest_city_tier("elsewhere", &towns, thresholds),
            0.0,
            "a region with no towns has no city lure"
        );
    }

    #[test]
    fn wonders_of_a_kin_culture_draw_their_heroes() {
        let data = GameData::load().unwrap();
        let mig = &data.balance.hero.migration;
        let land = region("aldervale", 60.0, 20.0, 40.0, 60.0);
        let scholarly_wonder = Landmark::from_seed(&LandmarkSeed {
            id: "w".to_owned(),
            name: "The Grand Athenaeum".to_owned(),
            region_id: "aldervale".to_owned(),
            culture: Culture::Scholarly,
            influence: 2.0,
        });
        let wonders = std::slice::from_ref(&scholarly_wonder);

        // A scholar is drawn more strongly to a land bearing a scholarly wonder...
        assert!(
            attractiveness(&land, wonders, 0.0, HeroRole::Scholar, mig)
                > attractiveness(&land, &[], 0.0, HeroRole::Scholar, mig),
            "a scholarly wonder should draw scholars"
        );
        // ...but a warrior, of a different culture, feels no such pull from it.
        assert_eq!(
            attractiveness(&land, wonders, 0.0, HeroRole::Warrior, mig),
            attractiveness(&land, &[], 0.0, HeroRole::Warrior, mig),
            "a scholarly wonder is no draw to a warrior"
        );
    }

    #[test]
    fn warriors_gather_where_scholars_flee() {
        let data = GameData::load().unwrap();
        let mut balance = data.balance.hero.clone();
        // Sample steady-state migration, not the death/aging system: let heroes
        // move often and live indefinitely so the distribution is what's tested.
        balance.move_chance = 0.5;
        balance.death.min_chance = 0.0;
        balance.death.elder_roll = 0.0;
        balance.death.danger_divisor = 1.0e9; // war would otherwise thin the warriors
        balance.life_expectancy_base = 1.0e6;
        let mut world = WorldState::new(&data);
        // Three regions so the weighted choice actually has alternatives.
        world.regions = vec![
            region("war", 30.0, 70.0, 20.0, 20.0),
            region("haven", 85.0, 10.0, 30.0, 85.0),
            region("wild", 45.0, 45.0, 40.0, 30.0),
        ];
        // Everyone starts in the neutral middle; roles should sort themselves out.
        world.heroes = (0..12)
            .map(|i| {
                let role = if i % 2 == 0 {
                    HeroRole::Warrior
                } else {
                    HeroRole::Scholar
                };
                hero(&format!("h{i}"), role, "wild")
            })
            .collect();

        for _ in 0..150 {
            tick_heroes(
                &mut world.heroes,
                &world.regions,
                &world.landmarks,
                &world.settlements,
                &data.balance.settlement.tier_thresholds,
                &mut world.rng,
                &balance,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
        }

        let warriors_in_war = world
            .heroes
            .iter()
            .filter(|h| h.is_alive && h.role == HeroRole::Warrior && h.region_id == "war")
            .count();
        let scholars_in_war = world
            .heroes
            .iter()
            .filter(|h| h.is_alive && h.role == HeroRole::Scholar && h.region_id == "war")
            .count();
        assert!(
            warriors_in_war > scholars_in_war,
            "warriors ({warriors_in_war}) should out-gather scholars ({scholars_in_war}) in the war region"
        );
    }

    #[test]
    fn renown_lowers_a_heros_danger_death() {
        let data = GameData::load().unwrap();
        let world = WorldState::new(&data);
        let mut famed = world.heroes[0].clone();
        famed.renown = 200.0;
        let mut unknown = famed.clone();
        unknown.renown = 0.0;
        assert!(
            danger_death_chance(&famed, 80.0, &data.balance.hero)
                < danger_death_chance(&unknown, 80.0, &data.balance.hero),
            "a renowned hero should be harder for danger to kill"
        );
    }

    #[test]
    fn renown_accrues_as_heroes_level() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        for _ in 0..100 {
            tick_heroes(
                &mut world.heroes,
                &world.regions,
                &world.landmarks,
                &world.settlements,
                &data.balance.settlement.tier_thresholds,
                &mut world.rng,
                &data.balance.hero,
                &mut world.chronicle,
                &data.strings.chronicle,
                world.year,
            );
        }
        assert!(
            world.heroes.iter().any(|h| h.renown > 0.0),
            "some hero should have earned renown by levelling"
        );
    }

    #[test]
    fn heroes_age_each_tick() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let before: Vec<u32> = world.heroes.iter().map(|h| h.age).collect();
        tick_heroes(
            &mut world.heroes,
            &world.regions,
            &world.landmarks,
            &world.settlements,
            &data.balance.settlement.tier_thresholds,
            &mut world.rng,
            &data.balance.hero,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
        for (hero, before_age) in world.heroes.iter().zip(before) {
            if hero.is_alive {
                assert_eq!(hero.age, before_age + 1);
            }
        }
    }

    #[test]
    fn simulation_is_deterministic_for_a_seed() {
        let data = GameData::load().unwrap();
        let run = || {
            let mut world = WorldState::new(&data);
            for _ in 0..50 {
                tick_heroes(
                    &mut world.heroes,
                    &world.regions,
                    &world.landmarks,
                    &world.settlements,
                    &data.balance.settlement.tier_thresholds,
                    &mut world.rng,
                    &data.balance.hero,
                    &mut world.chronicle,
                    &data.strings.chronicle,
                    world.year,
                );
            }
            world
                .heroes
                .iter()
                .map(|h| (h.level, h.age, h.is_alive, h.region_id.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }
}
