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
mod tests;
