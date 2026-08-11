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

/// A festival that passed into memory this tick: `(festival_name, region_id,
/// region_name)`, returned so the caller can enshrine it in a myth (GDD 5.2 <->
/// 6) — the festival's counterpart to a slain beast or a raised saint becoming
/// the stuff of folklore.
pub type FestivalRemembered = (String, String, String);

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
) -> Vec<FestivalRemembered> {
    // Advance every standing festival: while it runs it lifts its host and honours
    // the heroes who dwell there, then — its years spent — passes into memory. A
    // festival whose host has since been conquered away or sundered simply counts
    // down unremembered, its boons falling on no land.
    let mut remembered: Vec<FestivalRemembered> = Vec::new();
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
                        &[
                            ("festival", festival.name.clone()),
                            ("region", host_name.clone()),
                        ],
                    ),
                );
                remembered.push((festival.name.clone(), festival.region_id.clone(), host_name));
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
        return remembered;
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
    remembered
}

#[cfg(test)]
mod tests;
