//! Per-tick alliances (GDD 5.2): the standing pacts that form between regions
//! bound by a shared culture and the ties of trade. Kinship and commerce breed
//! amity among the peaceful; an alliance sheds a little of each partner's chaos —
//! the security of standing together — and it endures only while the two remain
//! of one character. Amity to war's enmity. Formation rolls through the world
//! RNG; the security dividend and dissolution are deterministic.

use crate::data::strings::ChronicleText;
use crate::data::{fill, PactBalance, RegionBalance};
use crate::world::{Chronicle, EventKind, Pact, Region, TradeRoute, War};
use macroquad_toolkit::rng::SeededRng;

#[allow(clippy::too_many_arguments)]
pub fn tick_pacts(
    pacts: &mut Vec<Pact>,
    regions: &mut [Region],
    routes: &[TradeRoute],
    wars: &[War],
    seq: &mut u64,
    balance: &PactBalance,
    region_balance: &RegionBalance,
    rng: &mut SeededRng,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    form_pacts(
        pacts, regions, routes, wars, seq, balance, rng, chronicle, text, year,
    );

    // The security dividend: each ally sheds a little chaos for standing together,
    // which also cools the belligerence that would drive it to war.
    for pact in pacts.iter_mut() {
        pact.age += 1;
        for id in [pact.region_a.clone(), pact.region_b.clone()] {
            if let Some(region) = regions.iter_mut().find(|r| r.id == id) {
                region.apply_deltas(0.0, -balance.chaos_relief, 0.0, 0.0, region_balance);
            }
        }
    }

    // An alliance lapses once its members no longer share a culture, or one of
    // them has passed from the map (conquered or sundered) — the kinship that
    // bound them is gone.
    pacts.retain(|pact| {
        let culture = |id: &str| regions.iter().find(|r| r.id == id).map(|r| r.culture);
        let (ca, cb) = (culture(&pact.region_a), culture(&pact.region_b));
        let holds = matches!((ca, cb), (Some(a), Some(b)) if a == b);
        if !holds {
            chronicle.push(
                year,
                EventKind::Region,
                fill(
                    &text.pact_dissolved,
                    &[
                        ("region_a", name_of(regions, &pact.region_a)),
                        ("region_b", name_of(regions, &pact.region_b)),
                    ],
                ),
            );
        }
        holds
    });
}

/// Forge fresh alliances between like-cultured, trade-linked, peaceable regions
/// (GDD 5.2).
#[allow(clippy::too_many_arguments)]
fn form_pacts(
    pacts: &mut Vec<Pact>,
    regions: &[Region],
    routes: &[TradeRoute],
    wars: &[War],
    seq: &mut u64,
    balance: &PactBalance,
    rng: &mut SeededRng,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    let peaceable = |r: &Region| r.chaos + r.danger < balance.form_max_belligerence;
    for i in 0..regions.len() {
        for j in (i + 1)..regions.len() {
            if pacts.len() >= balance.max_active {
                return;
            }
            let (a, b) = (&regions[i], &regions[j]);
            // Kinship of culture, ties of trade, and peace on both sides — and no
            // war or alliance already between them.
            if a.culture != b.culture
                || !peaceable(a)
                || !peaceable(b)
                || !trade_linked(routes, &a.id, &b.id)
                || wars
                    .iter()
                    .any(|w| w.aggressor_id == a.id && w.defender_id == b.id)
                || wars
                    .iter()
                    .any(|w| w.aggressor_id == b.id && w.defender_id == a.id)
                || pacts.iter().any(|p| p.binds(&a.id, &b.id))
            {
                continue;
            }
            if !rng.chance(balance.form_chance) {
                continue;
            }

            *seq += 1;
            pacts.push(Pact {
                id: format!("pact-{seq}"),
                region_a: a.id.clone(),
                region_b: b.id.clone(),
                age: 0,
            });
            chronicle.push(
                year,
                EventKind::Region,
                fill(
                    &text.pact_formed,
                    &[("region_a", a.name.clone()), ("region_b", b.name.clone())],
                ),
            );
        }
    }
}

/// Whether a trade route binds the two regions.
fn trade_linked(routes: &[TradeRoute], a: &str, b: &str) -> bool {
    routes
        .iter()
        .any(|r| (r.region_a == a && r.region_b == b) || (r.region_a == b && r.region_b == a))
}

fn name_of(regions: &[Region], id: &str) -> String {
    regions
        .iter()
        .find(|r| r.id == id)
        .map(|r| r.name.clone())
        .unwrap_or_else(|| id.to_owned())
}

#[cfg(test)]
mod tests;
