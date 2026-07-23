//! Per-tick myth behaviour (GDD 5.6): living myths echo across their region on
//! a cooldown, and fresh candidates are scored from region state — or seeded
//! directly when a hero passes into legend (`seed_hero_legend`) — so the player
//! always has tales to promote. Echoes are deterministic; candidate scoring uses
//! the world RNG for spread.

use crate::data::{fill, GameData, MythBalance, MythStat};
use crate::world::{Chronicle, EventKind, Hero, Myth, MythCandidate, Region};
use macroquad_toolkit::rng::SeededRng;

/// Echo mature myths and replenish candidates.
#[allow(clippy::too_many_arguments)]
pub fn tick_myths(
    myths: &mut Vec<Myth>,
    candidates: &mut Vec<MythCandidate>,
    seq: &mut u64,
    regions: &mut [Region],
    heroes: &mut [Hero],
    rng: &mut SeededRng,
    chronicle: &mut Chronicle,
    data: &GameData,
    year: u32,
) {
    let balance = &data.balance.myth;

    for myth in myths.iter_mut() {
        myth.echo_cooldown -= 1;

        // How vividly the myth's theme still lives in its home region (0 if the
        // region has been lost), read before the echo may nudge that stat.
        let region_fit = regions
            .iter()
            .find(|r| r.id == myth.region_id)
            .map(|r| region_stat(r, myth.stat))
            .unwrap_or(0.0);

        if myth.can_echo(balance.echo_threshold) {
            if let Some(region) = regions.iter_mut().find(|r| r.id == myth.region_id) {
                let (dp, dc, dd, dm) = stat_deltas(myth.stat, myth.stat_effect);
                region.apply_deltas(dp, dc, dd, dm, &data.balance.region);
                region.adjust_culture(myth.cultural_effect);
            }

            // The tale reaches the living, not only the land: every hero of its
            // home region takes heart and gains a little renown, scaled by how
            // vividly the myth still echoes — legend inspiring legend, the
            // counterpart to a hero's own passing seeding a myth (GDD 5.6 <-> 5.4).
            let inspiration = balance.echo_hero_renown * (myth.resonance / 100.0).clamp(0.0, 1.0);
            if inspiration > 0.0 {
                for hero in heroes
                    .iter_mut()
                    .filter(|h| h.is_alive && h.region_id == myth.region_id)
                {
                    hero.renown += inspiration;
                }
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
        // how deeply it was rooted when promoted — and by whether its subject
        // still thrives: a legend whose theme runs vivid in its land is kept
        // alive by a people who see it around them, while one whose region has
        // faded or fallen is forgotten fastest. Sustain never fully halts decay,
        // so every tale eventually passes.
        let sustain = 1.0 - (region_fit / 100.0).clamp(0.0, 1.0) * balance.resonance_sustain;
        myth.resonance = (myth.resonance - balance.resonance_decay * sustain).max(0.0);
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
        culture: theme.culture,
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
            culture: theme.culture,
            region_id: region_id.to_owned(),
            region_name: region_name.to_owned(),
            resonance: balance.resonance_max,
        },
    );
}

/// Seed a myth candidate commemorating the raising of a saint (GDD 5.1 <-> 5.6):
/// a mystical tale of holiness and sacrifice, rooted in the land that venerates
/// the saint, at full resonance. The faith counterpart to a beast-slaying's Valor
/// tale, so a land that raises saints grows mystical in memory. The player still
/// chooses whether to promote it; skipped once the board is saturated.
pub fn seed_saint_myth(
    candidates: &mut Vec<MythCandidate>,
    seq: &mut u64,
    saint_name: &str,
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
        .find(|t| t.id == balance.saint_theme_id)
        .or_else(|| data.myth_themes.first())
    else {
        return;
    };
    *seq += 1;
    candidates.insert(
        0,
        MythCandidate {
            id: format!("myth-{seq}"),
            title: fill(
                &data.strings.divine.saint_myth_title,
                &[("saint", saint_name.to_owned())],
            ),
            theme_name: theme.name.clone(),
            stat: theme.stat,
            cultural_effect: theme.cultural_effect,
            stat_effect: theme.stat_effect,
            culture: theme.culture,
            region_id: region_id.to_owned(),
            region_name: region_name.to_owned(),
            resonance: balance.resonance_max,
        },
    );
}

/// Seed a myth candidate remembering a great festival that has passed into memory
/// (GDD 5.2 <-> 6): a Triumph-tale of a realm's golden years, rooted in the land
/// that held the celebration and named for both festival and region, at full
/// resonance. So a land that celebrates its splendour grows storied for it, as one
/// that fells beasts grows martial and one that venerates its dead grows mystical.
/// The player still chooses whether to promote it; skipped once the board is
/// saturated so a run of festivals can't flood it.
pub fn seed_festival_myth(
    candidates: &mut Vec<MythCandidate>,
    seq: &mut u64,
    festival_name: &str,
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
        .find(|t| t.id == balance.festival_theme_id)
        .or_else(|| data.myth_themes.first())
    else {
        return;
    };
    *seq += 1;
    candidates.insert(
        0,
        MythCandidate {
            id: format!("myth-{seq}"),
            title: fill(
                &data.strings.divine.festival_myth_title,
                &[
                    ("festival", festival_name.to_owned()),
                    ("region", region_name.to_owned()),
                ],
            ),
            theme_name: theme.name.clone(),
            stat: theme.stat,
            cultural_effect: theme.cultural_effect,
            stat_effect: theme.stat_effect,
            culture: theme.culture,
            region_id: region_id.to_owned(),
            region_name: region_name.to_owned(),
            resonance: balance.resonance_max,
        },
    );
}

/// Seed a myth candidate commemorating a slain beast (GDD 5.2 <-> 5.6): a
/// Valor-tale of the hunt, rooted in the region where the beast fell and named
/// for both hero and beast, at full resonance. The same Valor theme a hero's
/// legend carries, so a land that fells beasts grows martial in memory. The
/// player still chooses whether to promote it; skipped once the board is
/// saturated so a season of hunts can't flood it.
pub fn seed_beast_myth(
    candidates: &mut Vec<MythCandidate>,
    seq: &mut u64,
    hero_name: &str,
    beast_name: &str,
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
    candidates.insert(
        0,
        MythCandidate {
            id: format!("myth-{seq}"),
            title: fill(
                &data.strings.divine.beast_myth_title,
                &[
                    ("hero", hero_name.to_owned()),
                    ("beast", beast_name.to_owned()),
                ],
            ),
            theme_name: theme.name.clone(),
            stat: theme.stat,
            cultural_effect: theme.cultural_effect,
            stat_effect: theme.stat_effect,
            culture: theme.culture,
            region_id: region_id.to_owned(),
            region_name: region_name.to_owned(),
            resonance: balance.resonance_max,
        },
    );
}

/// Seed a myth candidate born of a god crested to the height of wrath (GDD 5.6
/// pantheon <-> myths): the age remembers the divine gaze as a tale themed to
/// the deity's domain, rooted in the region where that domain burns brightest,
/// at full resonance. The tale's cultural shape is drawn from an authored theme
/// of the same domain, so a divine myth carries the same weight as any other.
/// Deterministic (no RNG); the player still chooses whether to promote it, and
/// it is skipped once the candidate pool is saturated so a stormy pantheon can't
/// flood the board.
pub fn seed_divine_myth(
    candidates: &mut Vec<MythCandidate>,
    seq: &mut u64,
    deity_name: &str,
    stat: MythStat,
    regions: &[Region],
    data: &GameData,
) {
    let balance = &data.balance.myth;
    if candidates.len() >= balance.candidate_count * 2 {
        return;
    }
    // A god's wrath is remembered where its domain runs strongest.
    let Some(region) = regions
        .iter()
        .max_by(|a, b| region_stat(a, stat).total_cmp(&region_stat(b, stat)))
    else {
        return;
    };
    let Some(theme) = data.myth_themes.iter().find(|t| t.stat == stat) else {
        return;
    };
    *seq += 1;
    candidates.insert(
        0,
        MythCandidate {
            id: format!("myth-{seq}"),
            title: fill(
                &data.strings.divine.divine_myth_title,
                &[
                    ("deity", deity_name.to_owned()),
                    ("region", region.name.clone()),
                ],
            ),
            theme_name: theme.name.clone(),
            stat,
            cultural_effect: theme.cultural_effect,
            stat_effect: theme.stat_effect,
            culture: theme.culture,
            region_id: region.id.clone(),
            region_name: region.name.clone(),
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
            &mut world.heroes,
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
    fn a_crested_god_is_remembered_in_myth_where_its_domain_burns() {
        // A god of danger crested to wrath seeds a Danger-themed candidate rooted
        // in the region where danger runs highest, attributed to the deity by
        // name and at full resonance (GDD 5.6 pantheon <-> myths).
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        world.myth_candidates.clear();
        world.regions.truncate(2);
        world.regions[0].danger = 20.0;
        world.regions[1].danger = 95.0;
        let dire_id = world.regions[1].id.clone();
        let dire_name = world.regions[1].name.clone();

        seed_divine_myth(
            &mut world.myth_candidates,
            &mut world.myth_seq,
            "Mordath",
            MythStat::Danger,
            &world.regions,
            &data,
        );

        assert_eq!(world.myth_candidates.len(), 1);
        let seeded = &world.myth_candidates[0]; // inserted at the front
        assert_eq!(seeded.stat, MythStat::Danger);
        assert_eq!(
            seeded.region_id, dire_id,
            "rooted where the domain burns brightest"
        );
        assert!(
            seeded.title.contains("Mordath") && seeded.title.contains(&dire_name),
            "the tale names the god and its land: {}",
            seeded.title
        );
        assert_eq!(
            seeded.resonance, data.balance.myth.resonance_max,
            "a divine tale rises at full resonance"
        );
    }

    #[test]
    fn divine_myths_stop_at_the_board_ceiling() {
        // However stormy the pantheon, divine myths never flood the board past
        // its ceiling — the player's promotion queue stays legible.
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let cap = data.balance.myth.candidate_count * 2;
        for _ in 0..cap * 2 {
            seed_divine_myth(
                &mut world.myth_candidates,
                &mut world.myth_seq,
                "Mordath",
                MythStat::Danger,
                &world.regions,
                &data,
            );
        }
        assert!(
            world.myth_candidates.len() <= cap,
            "divine myths overflowed the board ({} > {cap})",
            world.myth_candidates.len()
        );
    }

    #[test]
    fn a_slain_beast_becomes_a_valor_legend_of_the_hunt() {
        // Felling a beast seeds a Valor tale naming both hero and beast, rooted in
        // the region where it fell, at full resonance (GDD 5.2 <-> 5.6).
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        world.myth_candidates.clear();
        let region_id = world.regions[0].id.clone();
        let region_name = world.regions[0].name.clone();

        seed_beast_myth(
            &mut world.myth_candidates,
            &mut world.myth_seq,
            "Bramwell the Bold",
            "The Shadow Wyrm",
            &region_id,
            &region_name,
            &data,
        );

        assert_eq!(world.myth_candidates.len(), 1);
        let m = &world.myth_candidates[0];
        assert!(
            m.title.contains("Bramwell") && m.title.contains("Shadow Wyrm"),
            "the tale should name both hero and beast: {}",
            m.title
        );
        let legend_theme = data
            .myth_themes
            .iter()
            .find(|t| t.id == data.balance.myth.legend_theme_id)
            .unwrap();
        assert_eq!(
            m.culture, legend_theme.culture,
            "a tale of the hunt carries the Valor theme's culture"
        );
        assert_eq!(m.region_id, region_id);
        assert_eq!(m.resonance, data.balance.myth.resonance_max);
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
            culture: crate::data::Culture::Martial,
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
            &mut world.heroes,
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
    fn an_echoing_myth_inspires_the_heroes_of_its_land() {
        use crate::data::HeroRole;
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        world.regions.truncate(2);
        let home = world.regions[0].id.clone();
        let away = world.regions[1].id.clone();

        // A hero in the myth's home region, one in another, and a fallen one at
        // home — only the living local should be inspired by the echo.
        let hero = |id: &str, region: &str, alive: bool| Hero {
            id: id.to_owned(),
            name: id.to_owned(),
            role: HeroRole::Warrior,
            region_id: region.to_owned(),
            level: 3,
            age: 25,
            is_alive: alive,
            renown: 0.0,
        };
        world.heroes = vec![
            hero("local", &home, true),
            hero("distant", &away, true),
            hero("fallen", &home, false),
        ];

        // A vivid myth ready to echo this tick in the home region.
        world.myths.clear();
        world.myths.push(Myth {
            id: "m".to_owned(),
            title: "The Old Song".to_owned(),
            theme_name: "Valor".to_owned(),
            stat: MythStat::Prosperity,
            cultural_effect: 0.0,
            stat_effect: 0.0,
            culture: crate::data::Culture::Martial,
            region_id: home.clone(),
            region_name: world.regions[0].name.clone(),
            resonance: 100.0,
            echo_cooldown: 0,
        });

        tick_myths(
            &mut world.myths,
            &mut world.myth_candidates,
            &mut world.myth_seq,
            &mut world.regions,
            &mut world.heroes,
            &mut world.rng,
            &mut world.chronicle,
            &data,
            world.year,
        );

        let renown = |id: &str| world.heroes.iter().find(|h| h.id == id).unwrap().renown;
        assert!(
            renown("local") > 0.0,
            "a tale still sung should inspire the living heroes of its land"
        );
        assert_eq!(
            renown("distant"),
            0.0,
            "the tale reaches only its own region's heroes"
        );
        assert_eq!(renown("fallen"), 0.0, "the dead take no inspiration");
    }

    #[test]
    fn a_myth_endures_longer_where_its_theme_still_thrives() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        // Two regions: one drenched in magic, one barren of it — both hosting an
        // identical magic-myth. Silence their echoes (huge cooldown) so we
        // isolate the decay path, and hold the region stats fixed.
        world.regions.truncate(2);
        world.regions[0].magic_affinity = 100.0;
        world.regions[1].magic_affinity = 0.0;
        let vivid_id = world.regions[0].id.clone();
        let barren_id = world.regions[1].id.clone();
        let make = |region_id: &str, region_name: &str| Myth {
            id: format!("m-{region_id}"),
            title: "A Tale of Magic".to_owned(),
            theme_name: "Mystery".to_owned(),
            stat: MythStat::Magic,
            cultural_effect: 0.0,
            stat_effect: 0.0,
            culture: crate::data::Culture::Mystical,
            region_id: region_id.to_owned(),
            region_name: region_name.to_owned(),
            resonance: 80.0,
            echo_cooldown: 1_000_000,
        };
        world.myths.clear();
        world.myths.push(make(&vivid_id, "Vivid"));
        world.myths.push(make(&barren_id, "Barren"));

        // Barren decays 0.5/tick (80→25 in ~110 ticks); vivid decays at 40% of
        // that, so at 150 ticks the barren tale is gone and the vivid one holds.
        for _ in 0..150 {
            // Re-pin stats each tick in case any incidental drift occurs.
            world.regions[0].magic_affinity = 100.0;
            world.regions[1].magic_affinity = 0.0;
            tick_myths(
                &mut world.myths,
                &mut world.myth_candidates,
                &mut world.myth_seq,
                &mut world.regions,
                &mut world.heroes,
                &mut world.rng,
                &mut world.chronicle,
                &data,
                world.year,
            );
        }

        let vivid = world.myths.iter().find(|m| m.region_id == vivid_id);
        let barren = world.myths.iter().find(|m| m.region_id == barren_id);
        // The barren-land tale should have been forgotten first; the vivid one
        // still lingers in memory.
        assert!(
            barren.is_none(),
            "a tale whose theme has faded from its land should be forgotten sooner"
        );
        assert!(
            vivid.is_some(),
            "a tale whose theme still runs vivid should endure longer"
        );
    }

    #[test]
    fn a_faint_myth_fades_from_memory_and_frees_its_slot() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let floor = data.balance.myth.forgotten_floor;
        // A barren home (fit 0) so the tale gets no sustain and decays at full
        // rate — this test isolates the forgotten-floor removal, not sustain.
        world.regions[0].prosperity = 0.0;
        world.myths.clear();
        world.myths.push(Myth {
            id: "fading".to_owned(),
            title: "The Waning Tale".to_owned(),
            theme_name: "Valor".to_owned(),
            stat: MythStat::Prosperity,
            cultural_effect: 0.0,
            stat_effect: 0.0,
            culture: crate::data::Culture::Martial,
            region_id: world.regions[0].id.clone(),
            region_name: world.regions[0].name.clone(),
            resonance: floor + 0.4, // one full decay step from being forgotten
            echo_cooldown: 5,
        });

        tick_myths(
            &mut world.myths,
            &mut world.myth_candidates,
            &mut world.myth_seq,
            &mut world.regions,
            &mut world.heroes,
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
    fn a_saint_seeds_a_mystical_tale_of_holiness() {
        // Raising a saint seeds a mystical tale named for the saint, rooted in the
        // land that venerates them, carrying the sacrifice theme (GDD 5.1 <-> 5.6).
        let data = GameData::load().unwrap();
        let mut candidates: Vec<MythCandidate> = Vec::new();
        let mut seq = 0;
        seed_saint_myth(
            &mut candidates,
            &mut seq,
            "Saint Corvin",
            "aldermoor",
            "Aldermoor",
            &data,
        );
        assert_eq!(candidates.len(), 1);
        let m = &candidates[0];
        assert!(
            m.title.contains("Saint Corvin"),
            "the tale names its saint: {}",
            m.title
        );
        assert_eq!(m.region_id, "aldermoor");
        assert_eq!(
            m.culture,
            crate::data::Culture::Mystical,
            "a saint's tale is a mystical one, so a land of saints grows mystical in memory"
        );
        assert_eq!(m.resonance, data.balance.myth.resonance_max);
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
