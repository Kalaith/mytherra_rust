//! Per-tick vassalage (GDD 5.2): the tributary bonds a stronger region lays on a
//! weaker one. Vassalage is the political middle ground — between the equal amity
//! of a pact and the annexation of conquest. In peacetime a dominant region bends
//! a far weaker, trade-linked neighbour to its will; the vassal renders tribute of
//! its wealth to the overlord thereafter, and keeps its own existence under the
//! yoke until it has grown strong enough to throw it off. A region gathering many
//! vassals is an empire. Fully deterministic: might and eligibility are read from
//! world state, and a bond is sworn only on a fixed diplomatic cadence (never a
//! roll), so the system never perturbs the world's seeded RNG stream.

use crate::data::fill;
use crate::data::strings::ChronicleText;
use crate::data::{ConquestBalance, RegionBalance, VassalageBalance};
use crate::world::{resident_might, Chronicle, EventKind, Hero, Region, TradeRoute, Vassalage};

/// A region's total might: its base strength plus what its resident heroes lend —
/// the same reckoning conquest and war use to decide who prevails.
fn total_might(region: &Region, heroes: &[Hero], cb: &ConquestBalance) -> f32 {
    region.might(cb)
        + resident_might(
            heroes,
            &region.id,
            cb.might_per_hero_level,
            &cb.hero_might_weights,
        )
}

#[allow(clippy::too_many_arguments)]
pub fn tick_vassalages(
    vassalages: &mut Vec<Vassalage>,
    regions: &mut [Region],
    heroes: &[Hero],
    routes: &[TradeRoute],
    seq: &mut u64,
    balance: &VassalageBalance,
    conquest_balance: &ConquestBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    // Each region's total might this tick, so the whole system reads one snapshot.
    let mights: Vec<(String, f32)> = regions
        .iter()
        .map(|r| (r.id.clone(), total_might(r, heroes, conquest_balance)))
        .collect();
    let might_of = |id: &str| {
        mights
            .iter()
            .find(|(rid, _)| rid == id)
            .map(|(_, m)| *m)
            .unwrap_or(0.0)
    };

    // Rebellion, dissolution, and tribute on the standing bonds.
    let mut freed: Vec<(String, String, String)> = Vec::new(); // (overlord_name, vassal_name, vassal_id)
    vassalages.retain_mut(|v| {
        let overlord = regions.iter().find(|r| r.id == v.overlord_id);
        let vassal = regions.iter().find(|r| r.id == v.vassal_id);
        let (Some(overlord), Some(vassal)) = (overlord, vassal) else {
            // A partner has vanished — conquered away or sundered; the bond lapses.
            return false;
        };
        // A vassal grown strong enough throws off the yoke and rebels.
        if might_of(&v.vassal_id) >= might_of(&v.overlord_id) * balance.rebel_ratio {
            freed.push((
                overlord.name.clone(),
                vassal.name.clone(),
                v.vassal_id.clone(),
            ));
            return false;
        }
        v.age += 1;
        true
    });

    for (overlord_name, vassal_name, _vassal_id) in &freed {
        chronicle.push(
            year,
            EventKind::Region,
            fill(
                &text.vassalage_broken,
                &[
                    ("overlord", overlord_name.clone()),
                    ("vassal", vassal_name.clone()),
                ],
            ),
        );
    }

    // Tribute: each surviving bond drains a share of the vassal's wealth to its
    // overlord, some lost in the holding — vassalage moves wealth, and wastes a
    // little in the moving.
    let transfers: Vec<(String, String, f32)> = vassalages
        .iter()
        .filter_map(|v| {
            let vassal = regions.iter().find(|r| r.id == v.vassal_id)?;
            let tribute =
                (vassal.prosperity - balance.tribute_floor).max(0.0) * balance.tribute_fraction;
            (tribute > 0.0).then(|| (v.overlord_id.clone(), v.vassal_id.clone(), tribute))
        })
        .collect();
    for (overlord_id, vassal_id, tribute) in transfers {
        if let Some(v) = regions.iter_mut().find(|r| r.id == vassal_id) {
            v.apply_deltas(-tribute, 0.0, 0.0, 0.0, region_balance);
        }
        if let Some(o) = regions.iter_mut().find(|r| r.id == overlord_id) {
            o.apply_deltas(
                tribute * balance.tribute_efficiency,
                0.0,
                0.0,
                0.0,
                region_balance,
            );
        }
    }

    // Formation happens only on the diplomatic cadence — the slow reckonings at
    // which a subjugation, negotiated over years, is finally sworn. Between them,
    // nothing forms. Being a fixed cadence rather than a roll, it is fully
    // deterministic and never perturbs the world's seeded RNG stream.
    if balance.form_interval == 0 || !year.is_multiple_of(balance.form_interval) {
        return;
    }

    // An independent, dominant region may bend a far weaker, trade-linked neighbour
    // that is at peace (a region in crisis is conquered, not vassalized).
    let is_bound = |id: &str| vassalages.iter().any(|v| v.involves(id));
    let best = regions
        .iter()
        .filter(|o| !o.status.is_crisis() && !is_bound(&o.id))
        .flat_map(|overlord| {
            let overlord_might = might_of(&overlord.id);
            regions
                .iter()
                .filter(move |t| {
                    t.id != overlord.id
                        && !t.status.is_crisis()
                        && !is_bound(&t.id)
                        && routes
                            .iter()
                            .any(|r| r.touches(&overlord.id) && r.touches(&t.id))
                        && overlord_might >= might_of(&t.id) * balance.dominance_ratio
                })
                .map(move |t| (overlord, t))
        })
        // The most lopsided lawful pairing — the greatest gulf of might between the
        // would-be overlord and its would-be vassal — is the one that comes to pass.
        .max_by(|(oa, ta), (ob, tb)| {
            (might_of(&oa.id) - might_of(&ta.id))
                .total_cmp(&(might_of(&ob.id) - might_of(&tb.id)))
                .then_with(|| oa.id.cmp(&ob.id))
                .then_with(|| ta.id.cmp(&tb.id))
        });

    if let Some((overlord, target)) = best {
        *seq += 1;
        let (overlord_id, overlord_name) = (overlord.id.clone(), overlord.name.clone());
        let (vassal_id, vassal_name) = (target.id.clone(), target.name.clone());
        vassalages.push(Vassalage {
            id: format!("vassalage-{seq}"),
            overlord_id,
            vassal_id,
            age: 0,
        });
        chronicle.push(
            year,
            EventKind::Region,
            fill(
                &text.vassalage_sworn,
                &[("overlord", overlord_name), ("vassal", vassal_name)],
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::GameData;
    use crate::world::WorldState;

    #[allow(clippy::too_many_arguments)]
    fn run(world: &mut WorldState, data: &GameData) {
        tick_vassalages(
            &mut world.vassalages,
            &mut world.regions,
            &world.heroes,
            &world.trade_routes,
            &mut world.vassalage_seq,
            &data.balance.vassalage,
            &data.balance.conquest,
            &data.balance.region,
            &mut world.chronicle,
            &data.strings.chronicle,
            world.year,
        );
    }

    /// A world of exactly two trade-linked regions: a dominant `strong` and a far
    /// weaker `weak`, both at peace.
    fn overlord_and_weakling(data: &GameData) -> WorldState {
        let mut world = WorldState::new(data);
        // A year on the formation cadence, so a bond can be sworn this tick.
        world.year = 0;
        world.regions.truncate(2);
        world.heroes.clear();
        let (a, b) = (world.regions[0].id.clone(), world.regions[1].id.clone());
        // A trade road between them (the precondition for a bond).
        world.trade_routes.clear();
        world.trade_routes.push(crate::world::TradeRoute {
            id: "route".to_owned(),
            name: "Road".to_owned(),
            region_a: a.clone(),
            region_b: b.clone(),
            volume: 1.0,
        });
        // region[0] overwhelmingly strong, region[1] weak — both calm (at peace).
        world.regions[0].prosperity = 90.0;
        world.regions[0].population = 200_000.0;
        world.regions[0].chaos = 15.0;
        world.regions[0].danger = 15.0;
        world.regions[0].refresh_status(&data.balance.region);
        world.regions[1].prosperity = 40.0;
        world.regions[1].population = 3_000.0;
        world.regions[1].chaos = 15.0;
        world.regions[1].danger = 15.0;
        world.regions[1].refresh_status(&data.balance.region);
        world
    }

    #[test]
    fn the_strong_subordinate_the_weak_in_peacetime() {
        let data = GameData::load().unwrap();
        let mut world = overlord_and_weakling(&data);
        let strong = world.regions[0].id.clone();
        let weak = world.regions[1].id.clone();

        let mut sworn = false;
        for _ in 0..400 {
            run(&mut world, &data);
            if !world.vassalages.is_empty() {
                sworn = true;
                break;
            }
        }
        assert!(
            sworn,
            "a dominant region should vassalize a far weaker neighbour"
        );
        assert_eq!(world.vassalages[0].overlord_id, strong);
        assert_eq!(world.vassalages[0].vassal_id, weak);
    }

    #[test]
    fn a_vassal_renders_tribute_to_its_overlord() {
        let data = GameData::load().unwrap();
        let mut world = overlord_and_weakling(&data);
        let strong = world.regions[0].id.clone();
        let weak = world.regions[1].id.clone();
        // Seat the bond directly and hold the stats where tribute is owed.
        world.vassalages.push(Vassalage {
            id: "v".to_owned(),
            overlord_id: strong.clone(),
            vassal_id: weak.clone(),
            age: 1,
        });
        world.regions[1].prosperity = 80.0; // well above the tribute floor
        let vassal_before = world.regions[1].prosperity;
        run(&mut world, &data);
        let vassal_idx = world.regions.iter().position(|r| r.id == weak).unwrap();
        assert!(
            world.regions[vassal_idx].prosperity < vassal_before,
            "a vassal should render tribute, losing prosperity to its overlord"
        );
    }

    #[test]
    fn a_vassal_grown_strong_throws_off_the_yoke() {
        let data = GameData::load().unwrap();
        let mut world = overlord_and_weakling(&data);
        let strong = world.regions[0].id.clone();
        let weak = world.regions[1].id.clone();
        world.vassalages.push(Vassalage {
            id: "v".to_owned(),
            overlord_id: strong.clone(),
            vassal_id: weak.clone(),
            age: 5,
        });
        // The vassal rises to match its overlord — now it can rebel.
        world.regions[1].prosperity = 90.0;
        world.regions[1].population = 200_000.0;
        world.regions[1].danger = 15.0;
        run(&mut world, &data);
        assert!(
            world.vassalages.is_empty(),
            "a vassal as mighty as its overlord should rebel to independence"
        );
    }
}
