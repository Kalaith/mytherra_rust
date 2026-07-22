//! Per-tick inter-region war (GDD 5.2): the prolonged conflicts that break out
//! between regions and grind both down. A belligerent land — one wracked by chaos
//! and danger — falls upon the realm's richest region in envy; the war drains both
//! combatants year on year, the militarily weaker bleeding hardest, until it wanes
//! into a decisive rout or an exhausted stalemate. War fills the space between the
//! civilization system's one-sided rivalry and the outright annexation of
//! conquest: it does not remove a region, it wears one down, leaving the loser
//! scarred and ripe for the conquest that may follow. Ignition rolls through the
//! world RNG; the toll and resolution are deterministic.

use crate::data::strings::ChronicleText;
use crate::data::{fill, HeroRole, RegionBalance, WarBalance};
use crate::world::{Chronicle, EventKind, Hero, Region, Settlement, War};
use macroquad_toolkit::rng::SeededRng;

#[allow(clippy::too_many_arguments)]
pub fn tick_wars(
    wars: &mut Vec<War>,
    regions: &mut [Region],
    settlements: &mut [Settlement],
    heroes: &[Hero],
    seq: &mut u64,
    balance: &WarBalance,
    region_balance: &RegionBalance,
    rng: &mut SeededRng,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    ignite_wars(wars, regions, seq, balance, rng, chronicle, text, year);

    // Prosecute: each side suffers a base toll plus damage scaled by its
    // opponent's war might, and the war wanes toward its end.
    for war in wars.iter_mut() {
        war.age += 1;
        let aggressor_might = war_might(heroes, &war.aggressor_id);
        let defender_might = war_might(heroes, &war.defender_id);
        apply_toll(
            regions,
            settlements,
            &war.aggressor_id,
            defender_might,
            war.intensity,
            balance,
            region_balance,
        );
        apply_toll(
            regions,
            settlements,
            &war.defender_id,
            aggressor_might,
            war.intensity,
            balance,
            region_balance,
        );
        war.intensity -= balance.intensity_decay;
    }

    // Wars worn below the intensity floor have burned out and are decided.
    let ended: Vec<War> = wars
        .iter()
        .filter(|w| w.intensity < balance.min_intensity)
        .cloned()
        .collect();
    wars.retain(|w| w.intensity >= balance.min_intensity);
    for war in ended {
        resolve(
            &war,
            regions,
            heroes,
            balance,
            region_balance,
            chronicle,
            text,
            year,
        );
    }
}

/// A region's war might: the combined levels of its living Warriors and Rangers —
/// the martial strength it can bring to a war.
fn war_might(heroes: &[Hero], region_id: &str) -> f32 {
    heroes
        .iter()
        .filter(|h| {
            h.is_alive
                && h.region_id == region_id
                && matches!(h.role, HeroRole::Warrior | HeroRole::Ranger)
        })
        .map(|h| h.level as f32)
        .sum()
}

/// Declare fresh wars: a belligerent region falls upon the realm's richest other
/// region it isn't already fighting (GDD 5.2).
#[allow(clippy::too_many_arguments)]
fn ignite_wars(
    wars: &mut Vec<War>,
    regions: &[Region],
    seq: &mut u64,
    balance: &WarBalance,
    rng: &mut SeededRng,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    for i in 0..regions.len() {
        if wars.len() >= balance.max_active {
            break;
        }
        let belligerence = regions[i].chaos + regions[i].danger;
        if belligerence < balance.ignite_min_belligerence {
            continue;
        }
        // The richest other region it isn't already at war with — the crown it
        // envies. Ties break by id so the target is fixed.
        let Some(target) = regions
            .iter()
            .enumerate()
            .filter(|(j, r)| *j != i && !already_at_war(wars, &regions[i].id, &r.id))
            .max_by(|(_, a), (_, b)| {
                a.prosperity
                    .total_cmp(&b.prosperity)
                    .then_with(|| a.id.cmp(&b.id))
            })
            .map(|(j, _)| j)
        else {
            continue;
        };
        if !rng.chance(balance.ignite_chance) {
            continue;
        }

        *seq += 1;
        wars.push(War {
            id: format!("war-{seq}"),
            aggressor_id: regions[i].id.clone(),
            defender_id: regions[target].id.clone(),
            intensity: balance.start_intensity,
            age: 0,
        });
        chronicle.push(
            year,
            EventKind::Region,
            fill(
                &text.war_declared,
                &[
                    ("aggressor", regions[i].name.clone()),
                    ("defender", regions[target].name.clone()),
                ],
            ),
        );
    }
}

/// Whether two regions already have a war between them, either direction.
fn already_at_war(wars: &[War], a: &str, b: &str) -> bool {
    wars.iter().any(|w| {
        (w.aggressor_id == a && w.defender_id == b) || (w.aggressor_id == b && w.defender_id == a)
    })
}

/// The toll a war lays on one of its combatants this tick: a base drain of
/// prosperity and a rise in danger and chaos, plus extra harm scaled by the
/// opponent's war might, and a raid on its largest settlement.
#[allow(clippy::too_many_arguments)]
fn apply_toll(
    regions: &mut [Region],
    settlements: &mut [Settlement],
    region_id: &str,
    opponent_might: f32,
    intensity: f32,
    balance: &WarBalance,
    region_balance: &RegionBalance,
) {
    let damage = opponent_might * balance.might_damage * intensity;
    if let Some(region) = regions.iter_mut().find(|r| r.id == region_id) {
        region.apply_deltas(
            -(balance.prosperity_toll * intensity + damage),
            balance.chaos_toll * intensity,
            balance.danger_toll * intensity + damage,
            0.0,
            region_balance,
        );
    }
    if let Some(settlement) = largest_settlement(settlements, region_id) {
        let loss = settlement.population * balance.raid_population * intensity;
        settlement.population = (settlement.population - loss).max(0.0);
    }
}

/// Decide a burned-out war: the side with the greater war might prevails and
/// scars the loser, unless the two are within the stalemate margin, in which case
/// the war grinds to an exhausted draw (both already worn down by its toll).
#[allow(clippy::too_many_arguments)]
fn resolve(
    war: &War,
    regions: &mut [Region],
    heroes: &[Hero],
    balance: &WarBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    let aggressor_might = war_might(heroes, &war.aggressor_id);
    let defender_might = war_might(heroes, &war.defender_id);
    let name_of = |id: &str| {
        regions
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.name.clone())
            .unwrap_or_else(|| id.to_owned())
    };

    if (aggressor_might - defender_might).abs() <= balance.stalemate_margin {
        chronicle.push(
            year,
            EventKind::Region,
            fill(
                &text.war_stalemate,
                &[
                    ("aggressor", name_of(&war.aggressor_id)),
                    ("defender", name_of(&war.defender_id)),
                ],
            ),
        );
        return;
    }

    let (victor_id, loser_id) = if aggressor_might > defender_might {
        (&war.aggressor_id, &war.defender_id)
    } else {
        (&war.defender_id, &war.aggressor_id)
    };
    let victor_name = name_of(victor_id);
    let loser_name = name_of(loser_id);

    // The scar of defeat: the loser forfeits prosperity and takes on danger,
    // leaving it ripe for the conquest that may follow (GDD 5.2).
    if let Some(loser) = regions.iter_mut().find(|r| &r.id == loser_id) {
        loser.apply_deltas(
            -balance.loser_scar_prosperity,
            0.0,
            balance.loser_scar_danger,
            0.0,
            region_balance,
        );
    }
    chronicle.push(
        year,
        EventKind::Region,
        fill(
            &text.war_won,
            &[("victor", victor_name), ("loser", loser_name)],
        ),
    );
}

/// The region's most populous settlement, if any.
fn largest_settlement<'a>(
    settlements: &'a mut [Settlement],
    region_id: &str,
) -> Option<&'a mut Settlement> {
    settlements
        .iter_mut()
        .filter(|s| s.region_id == region_id)
        .max_by(|a, b| a.population.total_cmp(&b.population))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{GameData, HeroRole, HeroSeed};
    use crate::world::WorldState;

    fn warrior(id: &str, region_id: &str, level: u32) -> Hero {
        Hero::from_seed(&HeroSeed {
            id: id.to_owned(),
            name: id.to_owned(),
            role: HeroRole::Warrior,
            region_id: region_id.to_owned(),
            level,
            age: 30,
        })
    }

    fn run(world: &mut WorldState, data: &GameData, balance: &WarBalance) {
        tick_wars(
            &mut world.wars,
            &mut world.regions,
            &mut world.settlements,
            &world.heroes,
            &mut world.war_seq,
            balance,
            &data.balance.region,
            &mut world.rng,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
    }

    #[test]
    fn a_belligerent_land_makes_war_on_the_realms_richest() {
        // A region seething with chaos and danger declares war on the wealthiest
        // other region, not on a poorer one (GDD 5.2).
        let data = GameData::load().unwrap();
        let mut balance = data.balance.war.clone();
        balance.ignite_chance = 1.0; // certain this tick
        let mut world = WorldState::new(&data);
        world.wars.clear();
        // Region 0 is belligerent; region 2 is the richest of the rest.
        world.regions[0].chaos = 90.0;
        world.regions[0].danger = 90.0;
        for (i, r) in world.regions.iter_mut().enumerate() {
            r.prosperity = if i == 2 { 95.0 } else { 40.0 };
        }
        let aggressor = world.regions[0].id.clone();
        let richest = world.regions[2].id.clone();

        run(&mut world, &data, &balance);

        assert_eq!(world.wars.len(), 1, "a war should be declared");
        assert_eq!(world.wars[0].aggressor_id, aggressor);
        assert_eq!(
            world.wars[0].defender_id, richest,
            "the belligerent should strike at the realm's richest"
        );
    }

    #[test]
    fn a_settled_realm_stays_at_peace() {
        // Below the belligerence threshold, no war is declared however lucky the
        // roll.
        let data = GameData::load().unwrap();
        let mut balance = data.balance.war.clone();
        balance.ignite_chance = 1.0;
        let mut world = WorldState::new(&data);
        world.wars.clear();
        for r in &mut world.regions {
            r.chaos = 20.0;
            r.danger = 20.0;
        }
        run(&mut world, &data, &balance);
        assert!(world.wars.is_empty(), "a calm realm makes no war");
    }

    #[test]
    fn war_drains_both_combatants() {
        let data = GameData::load().unwrap();
        let mut balance = data.balance.war.clone();
        balance.ignite_chance = 0.0; // study the war we plant
        let mut world = WorldState::new(&data);
        let a = world.regions[0].id.clone();
        let b = world.regions[1].id.clone();
        world.regions[0].prosperity = 60.0;
        world.regions[1].prosperity = 60.0;
        let (pa, pb) = (world.regions[0].prosperity, world.regions[1].prosperity);
        let (da, db) = (world.regions[0].danger, world.regions[1].danger);
        world.wars.push(War {
            id: "w".to_owned(),
            aggressor_id: a,
            defender_id: b,
            intensity: 1.0,
            age: 0,
        });

        run(&mut world, &data, &balance);

        assert!(
            world.regions[0].prosperity < pa && world.regions[1].prosperity < pb,
            "war should drain both sides' prosperity"
        );
        assert!(
            world.regions[0].danger > da && world.regions[1].danger > db,
            "war should raise both sides' peril"
        );
    }

    #[test]
    fn the_mightier_side_prevails_and_scars_the_loser() {
        // A war between a martially strong aggressor and a weak defender ends with
        // the strong prevailing and the weak scarred (GDD 5.2).
        let data = GameData::load().unwrap();
        let mut balance = data.balance.war.clone();
        balance.ignite_chance = 0.0;
        balance.intensity_decay = 1.0; // burn out and resolve this tick
        let mut world = WorldState::new(&data);
        let strong = world.regions[0].id.clone();
        let weak = world.regions[1].id.clone();
        world
            .heroes
            .retain(|h| h.region_id != strong && h.region_id != weak);
        world.heroes.push(warrior("host", &strong, 40)); // strong host
        world.regions[1].prosperity = 60.0;
        let weak_prosperity_before = world.regions[1].prosperity;
        world.wars.push(War {
            id: "w".to_owned(),
            aggressor_id: strong.clone(),
            defender_id: weak.clone(),
            intensity: balance.min_intensity, // already at the floor; decays out
            age: 5,
        });

        run(&mut world, &data, &balance);

        assert!(world.wars.is_empty(), "the war should be resolved");
        assert!(
            world.regions[1].prosperity < weak_prosperity_before,
            "the defeated side should be scarred"
        );
        assert!(
            world
                .chronicle
                .iter_newest()
                .any(|e| e.message.contains("prevails")),
            "a decisive victory should be chronicled"
        );
    }
}
