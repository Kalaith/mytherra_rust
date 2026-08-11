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
use crate::data::{fill, ArtifactFocus, HeroRole, RegionBalance, WarBalance};
use crate::world::{
    Artifact, Chronicle, EventKind, Hero, Pact, Region, Settlement, Vassalage, War,
};
use macroquad_toolkit::rng::SeededRng;

#[allow(clippy::too_many_arguments)]
pub fn tick_wars(
    wars: &mut Vec<War>,
    regions: &mut [Region],
    settlements: &mut [Settlement],
    heroes: &[Hero],
    artifacts: &[Artifact],
    pacts: &[Pact],
    vassalages: &[Vassalage],
    seq: &mut u64,
    balance: &WarBalance,
    region_balance: &RegionBalance,
    rng: &mut SeededRng,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    ignite_wars(
        wars, regions, pacts, vassalages, seq, balance, rng, chronicle, text, year,
    );

    // Prosecute: each side suffers a base toll plus damage scaled by its
    // opponent's war might, and the war wanes toward its end.
    for war in wars.iter_mut() {
        war.age += 1;
        let aggressor_might = war_might(heroes, artifacts, pacts, &war.aggressor_id, balance);
        let defender_might = war_might(heroes, artifacts, pacts, &war.defender_id, balance);
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
            artifacts,
            pacts,
            balance,
            region_balance,
            chronicle,
            text,
            year,
        );
    }
}

/// A region's own war might: the combined levels of its living Warriors and
/// Rangers, plus the power of any War-focus artifacts bound to it — the martial
/// strength, mortal and divine, the land itself brings to a war (GDD 5.2 <-> 5.6).
fn base_might(
    heroes: &[Hero],
    artifacts: &[Artifact],
    region_id: &str,
    balance: &WarBalance,
) -> f32 {
    let martial: f32 = heroes
        .iter()
        .filter(|h| {
            h.is_alive
                && h.region_id == region_id
                && matches!(h.role, HeroRole::Warrior | HeroRole::Ranger)
        })
        .map(|h| h.level as f32)
        .sum();
    let relic: f32 = artifacts
        .iter()
        .filter(|a| a.focus == ArtifactFocus::War && a.region_id == region_id)
        .map(|a| a.power as f32 * balance.artifact_might)
        .sum();
    martial + relic
}

/// The full might a region can bring to a war: its own, plus the aid its sworn
/// allies send to its defence (GDD 5.2) — an alliance is a pledge to fight beside,
/// so a region with strong friends prevails where it would have fallen alone.
fn war_might(
    heroes: &[Hero],
    artifacts: &[Artifact],
    pacts: &[Pact],
    region_id: &str,
    balance: &WarBalance,
) -> f32 {
    let own = base_might(heroes, artifacts, region_id, balance);
    let aid: f32 = pacts
        .iter()
        .filter_map(|p| {
            if p.region_a == region_id {
                Some(p.region_b.as_str())
            } else if p.region_b == region_id {
                Some(p.region_a.as_str())
            } else {
                None
            }
        })
        .map(|ally| base_might(heroes, artifacts, ally, balance) * balance.ally_aid)
        .sum();
    own + aid
}

/// Declare fresh wars: a belligerent region falls upon the realm's richest other
/// region it isn't already fighting (GDD 5.2).
#[allow(clippy::too_many_arguments)]
fn ignite_wars(
    wars: &mut Vec<War>,
    regions: &[Region],
    pacts: &[Pact],
    vassalages: &[Vassalage],
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
        // The richest other region it isn't already at war with, nor allied to,
        // nor bound to in vassalage — one does not fall upon a sworn friend, and an
        // overlord does not war the vassal it protects, nor a vassal its master.
        // Ties break by id so the target is fixed.
        let Some(target) = regions
            .iter()
            .enumerate()
            .filter(|(j, r)| {
                *j != i
                    && !already_at_war(wars, &regions[i].id, &r.id)
                    && !pacts.iter().any(|p| p.binds(&regions[i].id, &r.id))
                    && !vassalages.iter().any(|v| v.binds(&regions[i].id, &r.id))
            })
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
    artifacts: &[Artifact],
    pacts: &[Pact],
    balance: &WarBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    let aggressor_might = war_might(heroes, artifacts, pacts, &war.aggressor_id, balance);
    let defender_might = war_might(heroes, artifacts, pacts, &war.defender_id, balance);
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
mod tests;
