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
mod tests;
