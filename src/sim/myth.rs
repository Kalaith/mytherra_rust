//! Per-tick myth behaviour (GDD 5.6): living myths echo across their region on
//! a cooldown, and fresh candidates are scored from region state — or seeded
//! directly when a hero passes into legend (`seed_hero_legend`) — so the player
//! always has tales to promote. Echoes are deterministic; candidate scoring uses
//! the world RNG for spread.

use crate::data::{fill, GameData, MythBalance, MythStat};
use crate::world::{Chronicle, EventKind, Myth, MythCandidate, Region};
use macroquad_toolkit::rng::SeededRng;

/// Echo mature myths and replenish candidates.
#[allow(clippy::too_many_arguments)]
pub fn tick_myths(
    myths: &mut Vec<Myth>,
    candidates: &mut Vec<MythCandidate>,
    seq: &mut u64,
    regions: &mut [Region],
    rng: &mut SeededRng,
    chronicle: &mut Chronicle,
    data: &GameData,
    year: u32,
) {
    let balance = &data.balance.myth;

    for myth in myths.iter_mut() {
        myth.echo_cooldown -= 1;
        if myth.can_echo(balance.echo_threshold) {
            if let Some(region) = regions.iter_mut().find(|r| r.id == myth.region_id) {
                let (dp, dc, dd, dm) = stat_deltas(myth.stat, myth.stat_effect);
                region.apply_deltas(dp, dc, dd, dm, &data.balance.region);
                region.adjust_culture(myth.cultural_effect);
            }
            myth.echo_cooldown = balance.echo_cooldown;
            chronicle.push(
                year,
                EventKind::System,
                fill(
                    &data.strings.chronicle.myth_echo,
                    &[
                        ("title", myth.title.clone()),
                        ("region", myth.region_name.clone()),
                    ],
                ),
            );
        } else if myth.echo_cooldown <= 0 {
            // Too faint to echo; wait another cooldown before re-checking.
            myth.echo_cooldown = balance.echo_cooldown;
        }

        // Every tale fades from living memory a little each year (GDD 5.6): its
        // resonance ebbs, so a myth first falls silent (below the echo
        // threshold) and eventually is forgotten. How long it lasts is set by
        // how deeply it was rooted when promoted.
        myth.resonance = (myth.resonance - balance.resonance_decay).max(0.0);
    }

    // Myths worn down past the forgotten floor pass out of memory, freeing a
    // slot on the capped roster for a new tale to rise.
    myths.retain(|m| {
        if m.resonance < balance.forgotten_floor {
            chronicle.push(
                year,
                EventKind::System,
                fill(
                    &data.strings.chronicle.myth_faded,
                    &[
                        ("title", m.title.clone()),
                        ("region", m.region_name.clone()),
                    ],
                ),
            );
            false
        } else {
            true
        }
    });

    let mut attempts = 0;
    while candidates.len() < balance.candidate_count && attempts < balance.candidate_count * 4 {
        attempts += 1;
        if let Some(candidate) = generate_candidate(seq, regions, rng, data) {
            candidates.push(candidate);
        }
    }
}

fn generate_candidate(
    seq: &mut u64,
    regions: &[Region],
    rng: &mut SeededRng,
    data: &GameData,
) -> Option<MythCandidate> {
    let balance = &data.balance.myth;
    // A legend is born where its subject runs vivid: pick the theme first, then
    // a region weighted by how strongly it embodies that theme's stat.
    let theme = rng.choose(&data.myth_themes)?.clone();
    let region = pick_region_by_theme(regions, theme.stat, balance, rng)?;

    // Resonance tracks that thematic fit, so a myth that truly belongs to its
    // land echoes stronger than one that barely fits.
    let fit = region_stat(region, theme.stat);
    let resonance = (fit * balance.resonance_scale
        + rng.range_f32(-balance.resonance_spread, balance.resonance_spread))
    .clamp(balance.resonance_min, balance.resonance_max);

    *seq += 1;
    Some(MythCandidate {
        id: format!("myth-{seq}"),
        title: fill(
            &data.strings.divine.new_myth_title,
            &[
                ("theme", theme.name.clone()),
                ("region", region.name.clone()),
            ],
        ),
        theme_name: theme.name.clone(),
        stat: theme.stat,
        cultural_effect: theme.cultural_effect,
        stat_effect: theme.stat_effect,
        region_id: region.id.clone(),
        region_name: region.name.clone(),
        resonance,
    })
}

/// Seed a myth candidate commemorating a hero who has just passed into legend
/// (GDD 5.4 <-> 5.6): a Valor-tale rooted in the hero's own region at full
/// resonance, since a legend needs no embellishment. The player still chooses
/// whether to promote it. Skipped once the candidate pool is saturated, so a run
/// of legends can't flood the board.
pub fn seed_hero_legend(
    candidates: &mut Vec<MythCandidate>,
    seq: &mut u64,
    hero_name: &str,
    region_id: &str,
    region_name: &str,
    data: &GameData,
) {
    let balance = &data.balance.myth;
    if candidates.len() >= balance.candidate_count * 2 {
        return;
    }
    let Some(theme) = data
        .myth_themes
        .iter()
        .find(|t| t.id == balance.legend_theme_id)
        .or_else(|| data.myth_themes.first())
    else {
        return;
    };
    *seq += 1;
    // Insert at the front so the fresh legend leads the board — the candidate
    // list is shown top-down and truncated, and a legend's tale shouldn't be the
    // one hidden below the fold.
    candidates.insert(
        0,
        MythCandidate {
            id: format!("myth-{seq}"),
            title: fill(
                &data.strings.divine.legend_myth_title,
                &[("hero", hero_name.to_owned())],
            ),
            theme_name: theme.name.clone(),
            stat: theme.stat,
            cultural_effect: theme.cultural_effect,
            stat_effect: theme.stat_effect,
            region_id: region_id.to_owned(),
            region_name: region_name.to_owned(),
            resonance: balance.resonance_max,
        },
    );
}

/// A region's value of the stat a myth theme is about.
fn region_stat(region: &Region, stat: MythStat) -> f32 {
    match stat {
        MythStat::Prosperity => region.prosperity,
        MythStat::Chaos => region.chaos,
        MythStat::Danger => region.danger,
        MythStat::Magic => region.magic_affinity,
    }
}

/// Pick a region weighted by how strongly it embodies the theme's stat, plus a
/// baseline floor so any region remains possible. Deterministic given the RNG.
fn pick_region_by_theme<'a>(
    regions: &'a [Region],
    stat: MythStat,
    balance: &MythBalance,
    rng: &mut SeededRng,
) -> Option<&'a Region> {
    if regions.is_empty() {
        return None;
    }
    let weight = |r: &Region| region_stat(r, stat) + balance.region_floor;
    let total: f32 = regions.iter().map(weight).sum();
    let mut roll = rng.next_f32() * total;
    for region in regions {
        roll -= weight(region);
        if roll <= 0.0 {
            return Some(region);
        }
    }
    regions.last()
}

/// Map a myth stat + amount onto (prosperity, chaos, danger, magic) deltas.
fn stat_deltas(stat: MythStat, amount: f32) -> (f32, f32, f32, f32) {
    match stat {
        MythStat::Prosperity => (amount, 0.0, 0.0, 0.0),
        MythStat::Chaos => (0.0, amount, 0.0, 0.0),
        MythStat::Danger => (0.0, 0.0, amount, 0.0),
        MythStat::Magic => (0.0, 0.0, 0.0, amount),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldState;

    #[test]
    fn candidates_replenish_to_target() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        tick_myths(
            &mut world.myths,
            &mut world.myth_candidates,
            &mut world.myth_seq,
            &mut world.regions,
            &mut world.rng,
            &mut world.chronicle,
            &data,
            world.year,
        );
        assert_eq!(
            world.myth_candidates.len(),
            data.balance.myth.candidate_count
        );
    }

    #[test]
    fn myths_favour_regions_that_embody_the_theme() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        // Two regions: one drenched in magic, one barren of it.
        world.regions.truncate(2);
        world.regions[0].magic_affinity = 100.0;
        world.regions[1].magic_affinity = 0.0;
        let magical_id = world.regions[0].id.clone();

        let mut rng = SeededRng::new(7);
        let mut in_magical = 0;
        for _ in 0..300 {
            let region = pick_region_by_theme(
                &world.regions,
                MythStat::Magic,
                &data.balance.myth,
                &mut rng,
            )
            .unwrap();
            if region.id == magical_id {
                in_magical += 1;
            }
        }
        // Floor 15 vs stat 100 → ~115/130 ≈ 88% land in the magical region.
        assert!(
            in_magical > 220,
            "magic myths should overwhelmingly favour the magical region ({in_magical}/300)"
        );
    }

    #[test]
    fn strong_myth_echoes_and_resets_cooldown() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let region_id = world.regions[0].id.clone();
        let region_name = world.regions[0].name.clone();
        let culture_before = world.regions[0].cultural_influence;
        world.myths.push(Myth {
            id: "m".to_owned(),
            title: "The Test".to_owned(),
            theme_name: "Valor".to_owned(),
            stat: MythStat::Prosperity,
            cultural_effect: 2.0,
            stat_effect: 1.0,
            region_id,
            region_name,
            resonance: 90.0,
            echo_cooldown: 0,
        });
        tick_myths(
            &mut world.myths,
            &mut world.myth_candidates,
            &mut world.myth_seq,
            &mut world.regions,
            &mut world.rng,
            &mut world.chronicle,
            &data,
            world.year,
        );
        assert!(world.regions[0].cultural_influence > culture_before);
        assert_eq!(
            world.myths[0].echo_cooldown,
            data.balance.myth.echo_cooldown
        );
    }

    #[test]
    fn a_faint_myth_fades_from_memory_and_frees_its_slot() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let floor = data.balance.myth.forgotten_floor;
        world.myths.clear();
        world.myths.push(Myth {
            id: "fading".to_owned(),
            title: "The Waning Tale".to_owned(),
            theme_name: "Valor".to_owned(),
            stat: MythStat::Prosperity,
            cultural_effect: 0.0,
            stat_effect: 0.0,
            region_id: world.regions[0].id.clone(),
            region_name: world.regions[0].name.clone(),
            resonance: floor + 0.4, // one decay step from being forgotten
            echo_cooldown: 5,
        });

        tick_myths(
            &mut world.myths,
            &mut world.myth_candidates,
            &mut world.myth_seq,
            &mut world.regions,
            &mut world.rng,
            &mut world.chronicle,
            &data,
            world.year,
        );

        assert!(
            !world.myths.iter().any(|m| m.id == "fading"),
            "a myth worn below the forgotten floor should pass out of memory"
        );
        assert!(
            world
                .chronicle
                .iter_newest()
                .any(|e| e.message.contains("The Waning Tale") && e.message.contains("fades")),
            "a myth's fading should be chronicled"
        );
    }

    #[test]
    fn a_legend_seeds_a_full_resonance_myth_in_its_own_land() {
        let data = GameData::load().unwrap();
        let mut candidates: Vec<MythCandidate> = Vec::new();
        let mut seq = 0;
        seed_hero_legend(
            &mut candidates,
            &mut seq,
            "Brogan",
            "kharzul",
            "Kharzul",
            &data,
        );
        assert_eq!(candidates.len(), 1);
        let m = &candidates[0];
        assert!(
            m.title.contains("Brogan"),
            "the tale names its hero: {}",
            m.title
        );
        assert_eq!(
            m.region_id, "kharzul",
            "the myth belongs to the hero's land"
        );
        assert_eq!(
            m.resonance, data.balance.myth.resonance_max,
            "a legend's tale rings at full resonance"
        );
    }

    #[test]
    fn a_saturated_board_refuses_more_legend_myths() {
        let data = GameData::load().unwrap();
        let ceiling = data.balance.myth.candidate_count * 2;
        let mut candidates: Vec<MythCandidate> = Vec::new();
        let mut seq = 0;
        // Fill past the ceiling, then confirm no further legend tale is added.
        for _ in 0..ceiling {
            seed_hero_legend(
                &mut candidates,
                &mut seq,
                "Hero",
                "kharzul",
                "Kharzul",
                &data,
            );
        }
        let saturated = candidates.len();
        seed_hero_legend(
            &mut candidates,
            &mut seq,
            "Late",
            "kharzul",
            "Kharzul",
            &data,
        );
        assert_eq!(candidates.len(), saturated, "the board can't be flooded");
    }
}
