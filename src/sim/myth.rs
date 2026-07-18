//! Per-tick myth behaviour (GDD 5.6): living myths echo across their region on
//! a cooldown, and fresh candidates are scored from region state so the player
//! always has tales to promote. Echoes are deterministic; candidate scoring uses
//! the world RNG for spread.

use crate::data::{fill, GameData, MythStat};
use crate::world::{Chronicle, EventKind, Myth, MythCandidate, Region};
use macroquad_toolkit::rng::SeededRng;

/// Echo mature myths and replenish candidates.
#[allow(clippy::too_many_arguments)]
pub fn tick_myths(
    myths: &mut [Myth],
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
    }

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
    let region = rng.choose(regions)?;
    let theme = rng.choose(&data.myth_themes)?.clone();
    let balance = &data.balance.myth;

    let metric = (region.prosperity + region.chaos + region.danger + region.magic_affinity) / 4.0;
    let resonance = (metric * balance.resonance_scale
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
}
