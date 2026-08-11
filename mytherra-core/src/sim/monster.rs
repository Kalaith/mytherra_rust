//! Per-tick bestiary (GDD 5.2): beasts emerge from perilous, untamed regions —
//! arcane horrors where magic runs strong, natural predators where the wilds are
//! merely dangerous — menace the land and raid its towns, grow into greater
//! terrors if left unopposed, and are hunted down by resident Warriors and
//! Rangers, whose mightiest earns the renown of the kill. The embodied threat
//! behind the abstract danger stat. Randomness (emergence) flows through the
//! world RNG; the menace, growth, and hunt are deterministic.

use crate::data::strings::ChronicleText;
use crate::data::{fill, HeroRole, MonsterBalance, MonsterType, RegionBalance};
use crate::world::{Chronicle, EventKind, Hero, Monster, Region, Settlement};
use macroquad_toolkit::rng::SeededRng;

/// One beast felled by a named hunter this tick: `(hero_name, beast_name,
/// region_id)`, returned so the caller can commemorate the deed in myth (GDD
/// 5.2 <-> 5.6) — the bestiary's counterpart to a hero passing into legend.
pub type BeastSlain = (String, String, String);

#[allow(clippy::too_many_arguments)]
pub fn tick_monster(
    monsters: &mut Vec<Monster>,
    regions: &mut [Region],
    settlements: &mut [Settlement],
    heroes: &mut [Hero],
    types: &[MonsterType],
    seq: &mut u64,
    balance: &MonsterBalance,
    region_balance: &RegionBalance,
    rng: &mut SeededRng,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) -> Vec<BeastSlain> {
    spawn_monsters(
        monsters, regions, heroes, types, seq, balance, rng, chronicle, text, year,
    );

    // Menace, growth, and the hunt.
    for monster in monsters.iter_mut() {
        monster.age += 1;
        let Some(ty) = types.iter().find(|t| t.id == monster.type_id) else {
            // Unknown kind (bestiary changed under an old save): let it wither.
            monster.ferocity -= balance.ferocity_growth;
            continue;
        };
        // A legendary terror ravages far beyond an ordinary beast: both its
        // per-tick menace and its raids on the towns are amplified.
        let menace = if monster.apex {
            balance.apex_menace_mult
        } else {
            1.0
        };
        // The beast makes the land perilous...
        if let Some(region) = regions.iter_mut().find(|r| r.id == monster.region_id) {
            region.apply_deltas(
                0.0,
                0.0,
                ty.danger_per_tick * monster.ferocity * menace,
                0.0,
                region_balance,
            );
        }
        // ...and raids the largest settlement for its people.
        if let Some(settlement) = largest_settlement(settlements, &monster.region_id) {
            let loss = settlement.population * ty.raid_population * monster.ferocity * menace;
            settlement.population = (settlement.population - loss).max(0.0);
        }
        // The hunt: resident hunters grind the beast down — but who can fight it
        // depends on its nature. Steel bites a natural predator full, but only
        // weakly bites an arcane horror, which a Mage must answer in kind. Left
        // wholly unopposed, the beast grows into a greater terror.
        let might = hunter_might(heroes, &monster.region_id, ty.arcane, balance);
        if might > 0.0 {
            monster.ferocity -= might * balance.slay_per_might;
        } else {
            monster.ferocity += balance.ferocity_growth;
        }

        // A beast grown fierce beyond all bound, unchallenged for an age, swells
        // into a named legendary terror — the mark of a land abandoned to the
        // wild. It happens once, when the threshold is first crossed.
        if !monster.apex
            && monster.ferocity >= balance.apex_ferocity
            && !text.monster_epithets.is_empty()
        {
            monster.apex = true;
            let epithet =
                text.monster_epithets[monster.age as usize % text.monster_epithets.len()].clone();
            monster.name = fill(
                &text.monster_ascends_name,
                &[
                    ("monster", monster.name.clone()),
                    ("epithet", epithet.clone()),
                ],
            );
            chronicle.push(
                year,
                EventKind::Region,
                fill(&text.monster_ascends, &[("monster", monster.name.clone())]),
            );
        }
    }

    // Beasts worn below the floor are slain (or, where no hunter remains, driven
    // off); the mightiest resident hunter claims the kill and its renown.
    let slain: Vec<(String, String, bool, bool)> = monsters
        .iter()
        .filter(|m| m.ferocity < balance.min_ferocity)
        .map(|m| {
            let arcane = types
                .iter()
                .find(|t| t.id == m.type_id)
                .is_some_and(|t| t.arcane);
            (m.region_id.clone(), m.name.clone(), arcane, m.apex)
        })
        .collect();
    monsters.retain(|m| m.ferocity >= balance.min_ferocity);

    let mut felled: Vec<BeastSlain> = Vec::new();
    for (region_id, name, arcane, apex) in slain {
        let slayer = heroes
            .iter_mut()
            .filter(|h| h.is_alive && h.region_id == region_id && hunts(h.role, arcane))
            .max_by_key(|h| h.level);
        match slayer {
            Some(hero) => {
                // Felling a legendary terror is the deed of a lifetime, worth far
                // more renown — and a chronicle line to match — than an ordinary kill.
                let (renown, line) = if apex {
                    (balance.apex_slay_renown, &text.monster_apex_slain)
                } else {
                    (balance.slay_renown, &text.monster_slain)
                };
                hero.renown += renown;
                let hero_name = hero.name.clone();
                chronicle.push(
                    year,
                    EventKind::Region,
                    fill(
                        line,
                        &[("hero", hero_name.clone()), ("monster", name.clone())],
                    ),
                );
                felled.push((hero_name, name, region_id));
            }
            None => chronicle.push(
                year,
                EventKind::Region,
                fill(&text.monster_driven_off, &[("monster", name)]),
            ),
        }
    }
    felled
}

/// Raise fresh beasts in perilous, untamed regions that have none (GDD 5.2).
#[allow(clippy::too_many_arguments)]
fn spawn_monsters(
    monsters: &mut Vec<Monster>,
    regions: &[Region],
    heroes: &[Hero],
    types: &[MonsterType],
    seq: &mut u64,
    balance: &MonsterBalance,
    rng: &mut SeededRng,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    if types.is_empty() {
        return;
    }
    for region in regions {
        if monsters.len() >= balance.max_active {
            break;
        }
        if region.danger < balance.emergence_min_danger
            || monsters.iter().any(|m| m.region_id == region.id)
        {
            continue;
        }
        // Peril breeds beasts, but two wardings hold them back: resident Rangers
        // who patrol the wilds (GDD 5.2 <-> 5.4), and the sacred ground of a devout
        // land, whose faith turns back the beast as surely as any spear (GDD 5.2
        // <-> 5.1) — so the more dangerous the land the likelier one stalks forth,
        // the more it is patrolled and the more it is hallowed the fewer do.
        let ward = ranger_ward(heroes, &region.id) * balance.ranger_ward;
        let hallowed = (region.divine_resonance - 50.0).max(0.0) * balance.resonance_ward;
        let chance = (balance.emergence_chance + region.danger * balance.emergence_danger_coeff
            - ward
            - hallowed)
            .max(0.0);
        if !rng.chance(chance) {
            continue;
        }
        // Arcane lands breed arcane horrors; the merely perilous breed predators.
        let arcane = region.magic_affinity >= balance.arcane_magic_threshold;
        let matching: Vec<&MonsterType> = types.iter().filter(|t| t.arcane == arcane).collect();
        let ty = if matching.is_empty() {
            &types[rng.below(types.len())]
        } else {
            matching[rng.below(matching.len())]
        };

        *seq += 1;
        let name = fill(
            &text.monster_name,
            &[("beast", ty.name.clone()), ("region", region.name.clone())],
        );
        monsters.push(Monster {
            id: format!("monster-{seq}"),
            name: name.clone(),
            type_id: ty.id.clone(),
            region_id: region.id.clone(),
            ferocity: ty.start_ferocity,
            age: 0,
            apex: false,
        });
        chronicle.push(
            year,
            EventKind::Region,
            fill(&text.monster_emergence, &[("monster", name)]),
        );
    }
}

/// Whether a hero of this role can meaningfully hunt a beast of this nature:
/// Warriors and Rangers face any predator, while a Mage joins the hunt only
/// against an arcane horror — magic answered in kind (GDD 5.2 <-> 5.4).
fn hunts(role: HeroRole, arcane: bool) -> bool {
    match role {
        HeroRole::Warrior | HeroRole::Ranger => true,
        HeroRole::Mage => arcane,
        _ => false,
    }
}

/// The combined levels of a region's living Rangers — the strength of its patrol
/// warding the wilds against beasts before they emerge.
fn ranger_ward(heroes: &[Hero], region_id: &str) -> f32 {
    heroes
        .iter()
        .filter(|h| h.is_alive && h.role == HeroRole::Ranger && h.region_id == region_id)
        .map(|h| h.level as f32)
        .sum()
}

/// The might a region can bring to bear against a beast, summed over its living
/// hunters and weighted by how well each answers the beast's nature: steel bites
/// a natural predator in full but an arcane horror only weakly, while a Mage is
/// the surest bane of the arcane and no help at all against a mundane beast.
fn hunter_might(heroes: &[Hero], region_id: &str, arcane: bool, balance: &MonsterBalance) -> f32 {
    heroes
        .iter()
        .filter(|h| h.is_alive && h.region_id == region_id)
        .map(|h| {
            let effectiveness = match h.role {
                HeroRole::Warrior | HeroRole::Ranger => {
                    if arcane {
                        balance.arcane_martial_effectiveness
                    } else {
                        1.0
                    }
                }
                HeroRole::Mage if arcane => balance.mage_arcane_effectiveness,
                _ => 0.0,
            };
            h.level as f32 * effectiveness
        })
        .sum()
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
