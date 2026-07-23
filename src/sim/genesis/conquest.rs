//! Conquest (GDD 5.2): a stable region annexes a trade-linked neighbour that
//! has collapsed into crisis and has no hero to defend it, merging the loser's
//! people and holdings into the victor and removing it from the map. The inverse
//! of a fracture — it removes a region rather than adding one.

use crate::data::strings::ChronicleText;
use crate::data::{fill, Agenda, ArtifactFocus, ConquestBalance, RegionBalance};
use crate::world::{
    Artifact, Chronicle, EventKind, Hero, Landmark, Pact, Region, RegionAgendas, ResourceNode,
    Settlement, TradeRoute, Vassalage, WeatherEvent,
};

/// Does a hero strong enough to hold the region against invasion live there? A
/// hero shields their home either by raw level or by a famous name (renown), so
/// a cultivated champion's earned fame guards its region even below the level bar.
fn has_defender(heroes: &[Hero], region_id: &str, balance: &ConquestBalance) -> bool {
    heroes.iter().any(|h| {
        h.is_alive
            && h.region_id == region_id
            && (h.level >= balance.defender_min_level || h.renown >= balance.defender_renown_min)
    })
}

/// A region's effective conquest might: its intrinsic might plus the raw
/// military power of any War artifacts bound to it — the player's offensive
/// lever over the genesis map (GDD 5.6 ↔ 5.2). A war relic thus both empowers a
/// region's conquests and hardens it against being conquered.
fn conquest_might(
    region: &Region,
    heroes: &[Hero],
    artifacts: &[Artifact],
    balance: &ConquestBalance,
) -> f32 {
    let war: f32 = artifacts
        .iter()
        .filter(|a| a.focus == ArtifactFocus::War && a.region_id == region.id)
        .map(|a| a.power as f32 * balance.artifact_war_might)
        .sum();
    let heroic = crate::world::resident_might(
        heroes,
        &region.id,
        balance.might_per_hero_level,
        &balance.hero_might_weights,
    );
    region.might(balance) + war + heroic
}

/// A region's full might to resist annexation: its own conquest might, plus the
/// aid its sworn allies lend to its defence (GDD 5.2). Allies stand against the
/// swallowing of a friend, so a land with mighty allies is harder to conquer —
/// used only for a *target*, since friends defend but do not help you annex.
fn defended_might(
    region: &Region,
    regions: &[Region],
    heroes: &[Hero],
    artifacts: &[Artifact],
    pacts: &[Pact],
    vassalages: &[Vassalage],
    balance: &ConquestBalance,
) -> f32 {
    let own = conquest_might(region, heroes, artifacts, balance);
    let aid: f32 = pacts
        .iter()
        .filter_map(|p| {
            if p.region_a == region.id {
                Some(p.region_b.as_str())
            } else if p.region_b == region.id {
                Some(p.region_a.as_str())
            } else {
                None
            }
        })
        .filter_map(|ally_id| regions.iter().find(|r| r.id == ally_id))
        .map(|ally| conquest_might(ally, heroes, artifacts, balance) * balance.ally_aid)
        .sum();
    // A vassal is shielded by the strength that holds it: its overlord marches to
    // its defence against any third power that would swallow it, lending might as a
    // sworn ally does (GDD 5.2). Tribute buys protection; the yoke is also a shield.
    let overlord_aid: f32 = vassalages
        .iter()
        .filter(|v| v.vassal_id == region.id)
        .filter_map(|v| regions.iter().find(|r| r.id == v.overlord_id))
        .map(|overlord| conquest_might(overlord, heroes, artifacts, balance) * balance.ally_aid)
        .sum();
    own + aid + overlord_aid
}

/// Is the region warded by a Protection artifact strong enough to turn back a
/// conquest? The player's divine lever over the genesis map (GDD 5.6 ↔ 5.2).
fn is_warded(artifacts: &[Artifact], region_id: &str, balance: &ConquestBalance) -> bool {
    artifacts.iter().any(|a| {
        a.focus == ArtifactFocus::Protection
            && a.region_id == region_id
            && a.power >= balance.shield_min_power
    })
}

/// The bonus a region's prevailing civilization course lends conquest: a Defense
/// target demands a wider margin to overrun (positive), a Rivalry aggressor will
/// forgo margin to strike (negative), any other course nothing (GDD 5.2 <-> 5.6).
fn agenda_margin(
    region: &Region,
    civ: &[RegionAgendas],
    agendas: &[Agenda],
    apply_threshold: f32,
    balance: &ConquestBalance,
) -> f32 {
    let Some(entry) = civ.iter().find(|c| c.region_id == region.id) else {
        return 0.0;
    };
    match crate::world::dominant_agenda(agendas, region, entry, apply_threshold) {
        Some(i) if agendas[i].id == "defense" => balance.defense_margin_bonus,
        Some(i) if agendas[i].id == "rivalry" => -balance.rivalry_aggression,
        _ => 0.0,
    }
}

/// The strongest aggressor / weakest eligible target pairing, if any conquest is
/// on. Deterministic: ranked by the might gap, ties broken toward earlier
/// regions.
#[allow(clippy::too_many_arguments)]
fn pick(
    regions: &[Region],
    heroes: &[Hero],
    trade_routes: &[TradeRoute],
    artifacts: &[Artifact],
    civ: &[RegionAgendas],
    agendas: &[Agenda],
    pacts: &[Pact],
    vassalages: &[Vassalage],
    apply_threshold: f32,
    balance: &ConquestBalance,
) -> Option<(usize, usize)> {
    if regions.len() <= balance.min_regions {
        return None;
    }
    let mut best: Option<(usize, usize, f32)> = None;
    for (ai, aggressor) in regions.iter().enumerate() {
        let a_might = conquest_might(aggressor, heroes, artifacts, balance);
        if aggressor.status.is_crisis() || a_might < balance.aggressor_min_might {
            continue;
        }
        for (ti, target) in regions.iter().enumerate() {
            // A people does not annex a sworn ally, and one is spared even in
            // crisis (GDD 5.2): the alliance stays the hand of conquest as it stays
            // the sword of war. Nor does an overlord devour its own vassal — it
            // holds it as a tributary, not a conquest, and protects it besides.
            if ti == ai
                || !target.status.is_crisis()
                || pacts.iter().any(|p| p.binds(&aggressor.id, &target.id))
                || vassalages
                    .iter()
                    .any(|v| v.binds(&aggressor.id, &target.id))
            {
                continue;
            }
            let gap = a_might
                - defended_might(
                    target, regions, heroes, artifacts, pacts, vassalages, balance,
                );
            // Both peoples' courses shape the margin: a Defense target demands a
            // wider gap and a Rivalry one lies more exposed; a Rivalry aggressor
            // will forgo margin to strike where a Defense-minded one holds off.
            let margin = balance.conquest_margin
                + agenda_margin(target, civ, agendas, apply_threshold, balance)
                + agenda_margin(aggressor, civ, agendas, apply_threshold, balance);
            if gap < margin
                || has_defender(heroes, &target.id, balance)
                || is_warded(artifacts, &target.id, balance)
            {
                continue;
            }
            if balance.require_trade_link
                && !trade_routes
                    .iter()
                    .any(|r| r.touches(&aggressor.id) && r.touches(&target.id))
            {
                continue;
            }
            if best.is_none_or(|(_, _, g)| gap > g) {
                best = Some((ai, ti, gap));
            }
        }
    }
    best.map(|(ai, ti, _)| (ai, ti))
}

/// Merge a crisis-stricken region into a stronger neighbour: transfer its people
/// and holdings, scar the victor with the cost of war, then remove it. At most
/// one conquest per tick.
#[allow(clippy::too_many_arguments)]
pub(super) fn run(
    regions: &mut Vec<Region>,
    settlements: &mut [Settlement],
    resource_nodes: &mut [ResourceNode],
    landmarks: &mut Vec<Landmark>,
    artifacts: &mut [Artifact],
    weather: &mut [WeatherEvent],
    heroes: &mut [Hero],
    trade_routes: &mut Vec<TradeRoute>,
    civilization: &mut Vec<RegionAgendas>,
    pacts: &[Pact],
    vassalages: &[Vassalage],
    agendas: &[Agenda],
    apply_threshold: f32,
    conquest_momentum: &mut f32,
    balance: &ConquestBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    let Some((winner_idx, loser_idx)) = pick(
        regions,
        heroes,
        trade_routes,
        artifacts,
        civilization,
        agendas,
        pacts,
        vassalages,
        apply_threshold,
        balance,
    ) else {
        return;
    };

    let winner_id = regions[winner_idx].id.clone();
    let winner_name = regions[winner_idx].name.clone();
    let loser_id = regions[loser_idx].id.clone();
    let loser_name = regions[loser_idx].name.clone();
    let spoils = regions[loser_idx].population * balance.population_transfer;

    // The war falls hardest on the loser's greatest city — the seat of
    // resistance is sacked as the region falls, its people scattered or slain
    // (GDD 5.2). Done before reassignment, while the loser's holdings are still
    // identifiable, and only if the region actually held a settlement.
    let sacked_city = settlements
        .iter_mut()
        .filter(|s| s.region_id == loser_id)
        .max_by(|a, b| a.population.total_cmp(&b.population))
        .map(|s| {
            s.population *= 1.0 - balance.sack_population_loss;
            s.prosperity = (s.prosperity - balance.sack_prosperity_loss).max(0.0);
            s.name.clone()
        });

    // The sack throws down the fallen realm's proudest wonder (GDD 5.2 <-> 5.7):
    // its grandest monument is razed as an example, with the last of its
    // defenders. Done before reassignment, while the loser still owns its wonders,
    // so only the greatest falls — the rest pass to the victor below.
    if balance.sack_razes_wonder {
        if let Some(idx) = landmarks
            .iter()
            .enumerate()
            .filter(|(_, l)| l.region_id == loser_id)
            .max_by(|(_, a), (_, b)| {
                a.stature
                    .total_cmp(&b.stature)
                    .then_with(|| a.id.cmp(&b.id))
            })
            .map(|(i, _)| i)
        {
            let razed = landmarks.remove(idx);
            chronicle.push(
                year,
                EventKind::Region,
                fill(
                    &text.landmark_sacked,
                    &[("landmark", razed.name), ("region", loser_name.clone())],
                ),
            );
        }
    }

    // Reassign everything the loser owned to its conqueror.
    for s in settlements.iter_mut() {
        if s.region_id == loser_id {
            s.region_id = winner_id.clone();
        }
    }
    for n in resource_nodes.iter_mut() {
        if n.region_id == loser_id {
            n.region_id = winner_id.clone();
        }
    }
    for l in landmarks.iter_mut() {
        if l.region_id == loser_id {
            l.region_id = winner_id.clone();
        }
    }
    for a in artifacts.iter_mut() {
        if a.region_id == loser_id {
            a.region_id = winner_id.clone();
        }
    }
    for w in weather.iter_mut() {
        if w.region_id == loser_id {
            w.region_id = winner_id.clone();
        }
    }
    for h in heroes.iter_mut() {
        if h.region_id == loser_id {
            h.region_id = winner_id.clone();
        }
    }
    // Trade routes fold onto the winner; any that would loop back are cut.
    for route in trade_routes.iter_mut() {
        if route.region_a == loser_id {
            route.region_a = winner_id.clone();
        }
        if route.region_b == loser_id {
            route.region_b = winner_id.clone();
        }
    }
    trade_routes.retain(|r| r.region_a != r.region_b);

    // The victor swells with absorbed population but pays the price of war.
    let winner = &mut regions[winner_idx];
    winner.population += spoils;
    winner.strife = 0.0;
    winner.apply_deltas(
        balance.winner_prosperity,
        balance.winner_chaos,
        balance.winner_danger,
        0.0,
        region_balance,
    );

    regions.retain(|r| r.id != loser_id);
    civilization.retain(|c| c.region_id != loser_id);

    // Feed the world's conquest momentum, which drives Conquest-era pressure.
    *conquest_momentum = (*conquest_momentum + balance.momentum_gain).min(balance.momentum_cap);

    chronicle.push(
        year,
        EventKind::Region,
        fill(
            &text.region_conquest,
            &[("winner", winner_name), ("loser", loser_name.clone())],
        ),
    );
    if let Some(city) = sacked_city {
        chronicle.push(
            year,
            EventKind::Region,
            fill(
                &text.region_sack,
                &[("settlement", city), ("region", loser_name)],
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{GameData, HeroRole};

    fn hero(region: &str, level: u32, renown: f32) -> Hero {
        Hero {
            id: "h".to_owned(),
            name: "H".to_owned(),
            role: HeroRole::Warrior,
            region_id: region.to_owned(),
            level,
            age: 30,
            is_alive: true,
            renown,
        }
    }

    #[test]
    fn a_famous_hero_defends_its_region_even_below_the_level_bar() {
        let balance = GameData::load().unwrap().balance.conquest;
        // A low-level but famous hero shields its region...
        let famous = vec![hero("aldermoor", 1, balance.defender_renown_min + 1.0)];
        assert!(has_defender(&famous, "aldermoor", &balance));
        // ...an equally-low unknown does not...
        let unknown = vec![hero("aldermoor", 1, 0.0)];
        assert!(!has_defender(&unknown, "aldermoor", &balance));
        // ...a seasoned hero shields regardless of renown...
        let veteran = vec![hero("aldermoor", balance.defender_min_level, 0.0)];
        assert!(has_defender(&veteran, "aldermoor", &balance));
        // ...and a defender guards only its own home.
        assert!(!has_defender(&famous, "kharzul", &balance));
    }

    #[test]
    fn a_regions_course_shapes_the_conquest_margin() {
        let data = GameData::load().unwrap();
        let balance = &data.balance.conquest;
        let threshold = data.balance.civilization.apply_threshold;
        let region = crate::world::WorldState::new(&data).regions[0].clone();
        let idx = |id: &str| data.agendas.iter().position(|a| a.id == id).unwrap();
        let with_course = |course: usize| {
            let mut entry = crate::world::RegionAgendas::new(region.id.clone(), data.agendas.len());
            entry.boosts[course] = 500.0;
            agenda_margin(&region, &[entry], &data.agendas, threshold, balance)
        };

        // Defense widens the required margin; Rivalry narrows it (bolder attacks,
        // a more exposed defence); another course does neither.
        assert!(
            (with_course(idx("defense")) - balance.defense_margin_bonus).abs() < f32::EPSILON,
            "a defense course should widen the margin"
        );
        assert!(
            (with_course(idx("rivalry")) + balance.rivalry_aggression).abs() < f32::EPSILON,
            "a rivalry course should narrow the margin"
        );
        assert_eq!(with_course(idx("recovery")), 0.0);
        // A region with no civilization entry contributes nothing.
        assert_eq!(
            agenda_margin(&region, &[], &data.agendas, threshold, balance),
            0.0
        );
    }
}
