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
mod tests;
