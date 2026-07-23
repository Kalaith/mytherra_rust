//! Applying a [`PlayerAction`] to the world — the authoritative command logic
//! (GDD 7.1). This is the game's rules for what a divine act does; the server
//! owns it, and the client's offline mode calls the very same [`apply`].
//!
//! Each handler mutates the shared [`WorldState`] and the acting player's
//! [`PlayerState`] (spending favor, moving stats, pushing chronicle entries) and
//! records player-facing [`Feedback`] in an [`ActionReport`] rather than driving
//! any UI directly — the client turns that report into notifications, the server
//! may log or ignore it. Authorization against a player's Standing happens
//! before `apply` is ever reached (§7.7).

mod action;
pub use action::PlayerAction;

use crate::data::{fill, ArtifactFocus, ChampionFocus, GameData};
use crate::world::{
    adjust_pressure, quote_event, weather_cost, Artifact, Bet, ConsequenceEffect,
    DelayedConsequence, EventKind, Myth, PlayerState, WeatherEvent, WorldState,
};

/// The severity of a piece of player-facing feedback from a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackLevel {
    Success,
    Warning,
    Info,
}

/// One player-facing message produced while applying a command.
#[derive(Debug, Clone)]
pub struct Feedback {
    pub level: FeedbackLevel,
    pub message: String,
}

/// The outcome of applying a command: the messages to surface to the player.
/// (World/player mutations are applied in place; this only carries feedback.)
#[derive(Debug, Clone, Default)]
pub struct ActionReport {
    pub feedback: Vec<Feedback>,
}

impl ActionReport {
    fn success(&mut self, message: String) {
        self.feedback.push(Feedback {
            level: FeedbackLevel::Success,
            message,
        });
    }

    fn warning(&mut self, message: String) {
        self.feedback.push(Feedback {
            level: FeedbackLevel::Warning,
            message,
        });
    }

    fn info(&mut self, message: String) {
        self.feedback.push(Feedback {
            level: FeedbackLevel::Info,
            message,
        });
    }
}

/// Apply an (already-authorized) command to the world and the acting player,
/// returning the feedback to show them (GDD 7.1).
pub fn apply(
    world: &mut WorldState,
    player: &mut PlayerState,
    data: &GameData,
    action: &PlayerAction,
) -> ActionReport {
    let mut report = ActionReport::default();
    match action {
        PlayerAction::RegionAction {
            region_id,
            action_id,
        } => region_action(world, player, data, region_id, action_id, &mut report),
        PlayerAction::DesignateChampion { hero_id } => {
            designate_champion(world, player, data, hero_id, &mut report)
        }
        PlayerAction::CultivateChampion { hero_id } => {
            cultivate_champion(world, player, data, hero_id, &mut report)
        }
        PlayerAction::SetChampionFocus { hero_id, focus } => {
            set_champion_focus(world, player, data, hero_id, *focus, &mut report)
        }
        PlayerAction::PlaceBet {
            event_id,
            confidence_index,
            stake_index,
        } => place_bet(
            world,
            player,
            data,
            event_id,
            *confidence_index,
            *stake_index,
            &mut report,
        ),
        PlayerAction::CreateArtifact { region_id, focus } => {
            create_artifact(world, player, data, region_id, *focus, &mut report)
        }
        PlayerAction::EmpowerArtifact { artifact_id } => {
            empower_artifact(world, player, data, artifact_id, &mut report)
        }
        PlayerAction::StabilizeArtifact { artifact_id } => {
            stabilize_artifact(world, player, data, artifact_id, &mut report)
        }
        PlayerAction::TransferArtifact {
            artifact_id,
            to_region_id,
        } => transfer_artifact(world, player, data, artifact_id, to_region_id, &mut report),
        PlayerAction::ShapeWeather {
            region_id,
            pattern_index,
            intensity_index,
        } => shape_weather(
            world,
            player,
            data,
            region_id,
            *pattern_index,
            *intensity_index,
            &mut report,
        ),
        PlayerAction::ResearchMagic { path_id } => {
            research_magic(world, player, data, path_id, &mut report)
        }
        PlayerAction::PromoteMyth { candidate_id } => {
            promote_myth(world, player, data, candidate_id, &mut report)
        }
        PlayerAction::AdvanceAgenda {
            region_id,
            agenda_index,
        } => advance_agenda(world, player, data, region_id, *agenda_index, &mut report),
        PlayerAction::AppeaseDeity { deity_id } => {
            appease_deity(world, player, data, deity_id, &mut report)
        }
        PlayerAction::ChallengeDeity { deity_id } => {
            challenge_deity(world, player, data, deity_id, &mut report)
        }
    }
    report
}

fn hero_name(world: &WorldState, hero_id: &str) -> String {
    world
        .heroes
        .iter()
        .find(|h| h.id == hero_id)
        .map(|h| h.name.clone())
        .unwrap_or_else(|| hero_id.to_owned())
}

fn region_action(
    world: &mut WorldState,
    player: &mut PlayerState,
    data: &GameData,
    region_id: &str,
    action_id: &str,
    report: &mut ActionReport,
) {
    let notes = &data.strings.notifications;
    let Some(def) = data.region_actions.get(action_id).cloned() else {
        report.warning(fill(&notes.unknown_action, &[("id", action_id.to_owned())]));
        return;
    };
    let Some(index) = world.regions.iter().position(|r| r.id == region_id) else {
        return;
    };
    let Some(region) = world.region(index) else {
        return;
    };
    let cost = region.action_cost(&def, &data.balance.region);
    if !player.spend(cost, &data.balance.player) {
        report.warning(notes.not_enough_favor.clone());
        return;
    }

    let year = world.year;
    let region_name;
    {
        let region = world.region_mut(index).expect("index checked above");
        region.apply_action(&def, &data.balance.region);
        region_name = region.name.clone();
    }
    let text = &data.strings;
    world.chronicle.push(
        year,
        EventKind::Divine,
        fill(
            &text.chronicle.divine_action,
            &[
                ("action", def.name.clone()),
                ("region", region_name.clone()),
            ],
        ),
    );
    report.success(fill(
        &text.notifications.action_success,
        &[
            ("action", def.name.clone()),
            ("region", region_name),
            ("cost", cost.to_string()),
        ],
    ));
}

fn designate_champion(
    world: &mut WorldState,
    player: &mut PlayerState,
    data: &GameData,
    hero_id: &str,
    report: &mut ActionReport,
) {
    let notes = &data.strings.notifications;
    let hero_name = match world.heroes.iter().find(|h| h.id == hero_id && h.is_alive) {
        Some(hero) => hero.name.clone(),
        None => return,
    };
    let balance = &data.balance.champion;
    if player.is_champion(hero_id) || player.champions.len() >= balance.max_roster {
        report.warning(notes.champion_designate_failed.clone());
        return;
    }
    if !player.spend(balance.designate_cost, &data.balance.player) {
        report.warning(notes.not_enough_favor.clone());
        return;
    }
    player.designate_champion(hero_id, ChampionFocus::Valor, &data.balance.champion);
    report.success(fill(&notes.champion_designated, &[("hero", hero_name)]));
}

fn cultivate_champion(
    world: &mut WorldState,
    player: &mut PlayerState,
    data: &GameData,
    hero_id: &str,
    report: &mut ActionReport,
) {
    let notes = &data.strings.notifications;
    let Some(cost) = player
        .champions
        .iter()
        .find(|c| c.hero_id == hero_id)
        .map(|c| c.cultivate_cost(&data.balance.champion))
    else {
        return;
    };
    let alive = world.heroes.iter().any(|h| h.id == hero_id && h.is_alive);
    if !alive {
        return;
    }
    if !player.spend(cost, &data.balance.player) {
        report.warning(notes.not_enough_favor.clone());
        return;
    }
    let gain = data.balance.champion.cultivate_bond_gain;
    if let Some(champion) = player.champion_mut(hero_id) {
        champion.bond += gain;
        champion.recompute_rank(&data.balance.champion);
    }
    let name = hero_name(world, hero_id);
    report.success(fill(&notes.champion_cultivated, &[("hero", name)]));
}

fn set_champion_focus(
    world: &mut WorldState,
    player: &mut PlayerState,
    data: &GameData,
    hero_id: &str,
    focus: ChampionFocus,
    report: &mut ActionReport,
) {
    let Some(champion) = player.champion_mut(hero_id) else {
        return;
    };
    champion.focus = focus;
    let name = hero_name(world, hero_id);
    report.info(fill(
        &data.strings.notifications.champion_focus_changed,
        &[("hero", name), ("focus", focus.label().to_owned())],
    ));
}

fn place_bet(
    world: &mut WorldState,
    player: &mut PlayerState,
    data: &GameData,
    event_id: &str,
    confidence_index: usize,
    stake_index: usize,
    report: &mut ActionReport,
) {
    let notes = &data.strings.notifications;
    let betting = &data.balance.betting;
    let stake = betting.stake_presets[stake_index.min(betting.stake_presets.len() - 1)];
    let confidence =
        data.confidence_levels[confidence_index.min(data.confidence_levels.len() - 1)].clone();

    let Some(idx) = world
        .speculations
        .iter()
        .position(|e| e.id == event_id && e.is_active())
    else {
        report.warning(notes.bet_closed.clone());
        return;
    };

    let (quote, target_name, bet_type_name, deadline, predicate) = {
        let event = &world.speculations[idx];
        let era_progress = world.era.pressure / data.balance.era.breaking_threshold.max(1.0);
        let likelihood = event.likelihood(
            &world.heroes,
            &world.regions,
            &world.settlements,
            era_progress,
        );
        let quote = quote_event(event, likelihood, &confidence, stake, &data.balance.betting);
        (
            quote,
            event.target_name.clone(),
            event.bet_type_name.clone(),
            event.deadline_year,
            event.predicate,
        )
    };

    if !player.place_stake(stake) {
        report.warning(notes.bet_unaffordable.clone());
        return;
    }
    // The player joins the "yes" side, shifting the crowd lean for later bets.
    world.speculations[idx].crowd_yes += stake as f32;

    player.bets.push(Bet {
        event_id: event_id.to_owned(),
        predicate,
        bet_type_name,
        target_name: target_name.clone(),
        confidence_name: confidence.name.clone(),
        stake,
        potential_payout: quote.payout,
        odds: quote.odds,
        placed_year: world.year,
        deadline_year: deadline,
        resolved: None,
    });
    report.success(fill(
        &notes.bet_placed,
        &[("target", target_name), ("stake", stake.to_string())],
    ));
}

fn create_artifact(
    world: &mut WorldState,
    player: &mut PlayerState,
    data: &GameData,
    region_id: &str,
    focus: ArtifactFocus,
    report: &mut ActionReport,
) {
    let notes = &data.strings.notifications;
    let balance = &data.balance.artifact;
    if world.artifacts.len() >= balance.max_active {
        report.warning(notes.artifact_max.clone());
        return;
    }
    if !player.spend(balance.create_cost, &data.balance.player) {
        report.warning(notes.not_enough_favor.clone());
        return;
    }

    world.artifact_seq += 1;
    let seq = world.artifact_seq;
    let name = fill(
        &data.strings.divine.new_artifact_name,
        &[("focus", focus.label().to_owned()), ("n", seq.to_string())],
    );
    world.artifacts.push(Artifact {
        id: format!("art-{seq}"),
        name: name.clone(),
        focus,
        power: 1,
        instability: 0.0,
        region_id: region_id.to_owned(),
    });
    report.success(fill(&notes.artifact_created, &[("name", name)]));
}

fn empower_artifact(
    world: &mut WorldState,
    player: &mut PlayerState,
    data: &GameData,
    id: &str,
    report: &mut ActionReport,
) {
    let notes = &data.strings.notifications;
    let Some(cost) = world
        .artifacts
        .iter()
        .find(|a| a.id == id)
        .map(|a| a.empower_cost(&data.balance.artifact))
    else {
        return;
    };
    if !player.spend(cost, &data.balance.player) {
        report.warning(notes.not_enough_favor.clone());
        return;
    }
    let gain = data.balance.artifact.empower_instability_gain;
    if let Some(artifact) = world.artifacts.iter_mut().find(|a| a.id == id) {
        artifact.power += 1;
        artifact.instability += gain;
        let name = artifact.name.clone();
        report.success(fill(&notes.artifact_empowered, &[("name", name)]));
    }
}

fn stabilize_artifact(
    world: &mut WorldState,
    player: &mut PlayerState,
    data: &GameData,
    id: &str,
    report: &mut ActionReport,
) {
    let notes = &data.strings.notifications;
    let balance = &data.balance.artifact;
    if !world.artifacts.iter().any(|a| a.id == id) {
        return;
    }
    if !player.spend(balance.stabilize_cost, &data.balance.player) {
        report.warning(notes.not_enough_favor.clone());
        return;
    }
    let amount = data.balance.artifact.stabilize_amount;
    if let Some(artifact) = world.artifacts.iter_mut().find(|a| a.id == id) {
        artifact.instability = (artifact.instability - amount).max(0.0);
        let name = artifact.name.clone();
        report.success(fill(&notes.artifact_stabilized, &[("name", name)]));
    }
}

fn transfer_artifact(
    world: &mut WorldState,
    player: &mut PlayerState,
    data: &GameData,
    id: &str,
    to_region_id: &str,
    report: &mut ActionReport,
) {
    let notes = &data.strings.notifications;
    let balance = &data.balance.artifact;
    if !world.artifacts.iter().any(|a| a.id == id) {
        return;
    }
    let Some(next_name) = world
        .regions
        .iter()
        .find(|r| r.id == to_region_id)
        .map(|r| r.name.clone())
    else {
        return;
    };
    if !player.spend(balance.transfer_cost, &data.balance.player) {
        report.warning(notes.not_enough_favor.clone());
        return;
    }
    let transfer_instability = balance.transfer_instability;
    if let Some(artifact) = world.artifacts.iter_mut().find(|a| a.id == id) {
        artifact.region_id = to_region_id.to_owned();
        // Wrenching a bound relic loose unsettles it — the journey adds
        // instability, so moving is a considered act, not a free reposition.
        artifact.instability += transfer_instability;
        let name = artifact.name.clone();
        report.success(fill(
            &notes.artifact_transferred,
            &[("name", name), ("region", next_name)],
        ));
    }
}

fn appease_deity(
    world: &mut WorldState,
    player: &mut PlayerState,
    data: &GameData,
    id: &str,
    report: &mut ActionReport,
) {
    let amount = data.balance.pantheon.appease_amount;
    let cost = data.balance.pantheon.appease_cost;
    influence_deity(world, player, data, id, -amount, -1.0, cost, true, report);
}

fn challenge_deity(
    world: &mut WorldState,
    player: &mut PlayerState,
    data: &GameData,
    id: &str,
    report: &mut ActionReport,
) {
    let amount = data.balance.pantheon.challenge_amount;
    let cost = data.balance.pantheon.challenge_cost;
    influence_deity(world, player, data, id, amount, 1.0, cost, false, report);
}

/// Shared appease/challenge logic: move the target's pressure, ripple the
/// opposite way to its ally (`ripple_sign`) and rival, honouring the
/// relationship cooldown (GDD 5.6).
#[allow(clippy::too_many_arguments)]
fn influence_deity(
    world: &mut WorldState,
    player: &mut PlayerState,
    data: &GameData,
    id: &str,
    target_delta: f32,
    ripple_sign: f32,
    cost: i64,
    appease: bool,
    report: &mut ActionReport,
) {
    let notes = &data.strings.notifications;
    let Some((ally_id, rival_id, name, on_cooldown)) =
        world.pantheon.iter().find(|d| d.id == id).map(|d| {
            (
                d.ally_id.clone(),
                d.rival_id.clone(),
                d.name.clone(),
                d.cooldown > 0,
            )
        })
    else {
        return;
    };
    if on_cooldown {
        report.warning(fill(&notes.deity_cooldown, &[("deity", name)]));
        return;
    }
    if !player.spend(cost, &data.balance.player) {
        report.warning(notes.not_enough_favor.clone());
        return;
    }

    let ripple = data.balance.pantheon.ripple;
    let cooldown = data.balance.pantheon.cooldown;
    adjust_pressure(&mut world.pantheon, id, target_delta);
    adjust_pressure(&mut world.pantheon, &ally_id, ripple * ripple_sign);
    adjust_pressure(&mut world.pantheon, &rival_id, -ripple * ripple_sign);
    if let Some(deity) = world.pantheon.iter_mut().find(|d| d.id == id) {
        deity.cooldown = cooldown;
    }

    let template = if appease {
        &notes.deity_appeased
    } else {
        &notes.deity_challenged
    };
    report.success(fill(template, &[("deity", name)]));
}

fn advance_agenda(
    world: &mut WorldState,
    player: &mut PlayerState,
    data: &GameData,
    region_id: &str,
    agenda_index: usize,
    report: &mut ActionReport,
) {
    let notes = &data.strings.notifications;
    if agenda_index >= data.agendas.len() {
        return;
    }
    let Some(region_name) = world
        .regions
        .iter()
        .find(|r| r.id == region_id)
        .map(|r| r.name.clone())
    else {
        return;
    };

    let on_cooldown = world
        .civilization
        .iter()
        .find(|e| e.region_id == region_id)
        .map(|e| e.cooldown > 0)
        .unwrap_or(true);
    if on_cooldown {
        report.warning(notes.agenda_cooldown.clone());
        return;
    }

    let cost = data.balance.civilization.advance_cost;
    if !player.spend(cost, &data.balance.player) {
        report.warning(notes.not_enough_favor.clone());
        return;
    }
    let boost = data.balance.civilization.advance_boost;
    let cooldown = data.balance.civilization.advance_cooldown;
    if let Some(entry) = world
        .civilization
        .iter_mut()
        .find(|e| e.region_id == region_id)
    {
        if let Some(value) = entry.boosts.get_mut(agenda_index) {
            *value += boost;
        }
        entry.cooldown = cooldown;
    }
    let agenda_name = data.agendas[agenda_index].name.clone();
    report.success(fill(
        &notes.agenda_advanced,
        &[("agenda", agenda_name), ("region", region_name)],
    ));
}

fn promote_myth(
    world: &mut WorldState,
    player: &mut PlayerState,
    data: &GameData,
    id: &str,
    report: &mut ActionReport,
) {
    let notes = &data.strings.notifications;
    if world.myths.len() >= data.balance.myth.cap {
        report.warning(notes.myth_cap.clone());
        return;
    }
    let Some(pos) = world.myth_candidates.iter().position(|c| c.id == id) else {
        return;
    };
    let cost = data.balance.myth.promote_cost;
    if !player.spend(cost, &data.balance.player) {
        report.warning(notes.not_enough_favor.clone());
        return;
    }
    let cooldown = data.balance.myth.echo_cooldown;
    let candidate = world.myth_candidates.remove(pos);
    let title = candidate.title.clone();
    world.myths.push(Myth::from_candidate(&candidate, cooldown));
    report.success(fill(&notes.myth_promoted, &[("title", title)]));
}

fn research_magic(
    world: &mut WorldState,
    player: &mut PlayerState,
    data: &GameData,
    id: &str,
    report: &mut ActionReport,
) {
    let notes = &data.strings.notifications;
    if !world.magic_paths.iter().any(|p| p.id == id) {
        return;
    }
    let cost = data.balance.magic.research_cost;
    if !player.spend(cost, &data.balance.player) {
        report.warning(notes.not_enough_favor.clone());
        return;
    }
    let pgain = data.balance.magic.research_progress_gain;
    let egain = data.balance.magic.research_evidence_gain;
    let cap = data.balance.magic.stat_cap;
    if let Some(path) = world.magic_paths.iter_mut().find(|p| p.id == id) {
        path.progress = (path.progress + pgain).min(cap);
        path.evidence = (path.evidence + egain).min(cap);
        path.recompute_state(&data.balance.magic);
        let name = path.name.clone();
        report.success(fill(&notes.magic_researched, &[("path", name)]));
    }
}

#[allow(clippy::too_many_arguments)]
fn shape_weather(
    world: &mut WorldState,
    player: &mut PlayerState,
    data: &GameData,
    region_id: &str,
    pattern_index: usize,
    intensity_index: usize,
    report: &mut ActionReport,
) {
    let notes = &data.strings.notifications;
    if world.weather.len() >= data.balance.weather.max_active {
        report.warning(notes.weather_max.clone());
        return;
    }
    let pattern = data.weather_patterns[pattern_index.min(data.weather_patterns.len() - 1)].clone();
    let intensity =
        data.weather_intensities[intensity_index.min(data.weather_intensities.len() - 1)].clone();
    let Some((region_id, region_name, cost)) =
        world.regions.iter().find(|r| r.id == region_id).map(|r| {
            let cost = weather_cost(
                data.balance.weather.base_cost,
                intensity.cost_mult,
                r.cost_multiplier(&data.balance.region),
            );
            (r.id.clone(), r.name.clone(), cost)
        })
    else {
        return;
    };
    if !player.spend(cost, &data.balance.player) {
        report.warning(notes.not_enough_favor.clone());
        return;
    }
    // Shaped weather leaves a delayed mark: a harmful working (net loss of
    // prosperity) scars with flood or famine, a fair one ripens into a later
    // harvest — both unfolding via the consequence queue (GDD 5.6).
    let wb = &data.balance.weather;
    let aftermath = if pattern.prosperity < 0.0 {
        Some(ConsequenceEffect::SettlementBlight(
            wb.aftermath_blight * intensity.magnitude,
        ))
    } else if pattern.prosperity > 0.0 {
        Some(ConsequenceEffect::SettlementBloom(
            wb.aftermath_bloom * intensity.magnitude,
        ))
    } else {
        None
    };
    if let Some(effect) = aftermath {
        world.pending_consequences.push(DelayedConsequence {
            region_id: region_id.clone(),
            source: pattern.name.clone(),
            delay: wb.aftermath_delay,
            effect,
        });
    }
    world
        .weather
        .push(WeatherEvent::from_parts(region_id, &pattern, &intensity));
    report.success(fill(
        &notes.weather_shaped,
        &[("pattern", pattern.name), ("region", region_name)],
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_region_action_spends_favor_and_moves_the_land() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut player = PlayerState::new(&data.config);
        let region_id = world.regions[0].id.clone();
        let action_id = data.ordered_region_actions()[0].id.clone();
        let favor_before = player.favor;

        let report = apply(
            &mut world,
            &mut player,
            &data,
            &PlayerAction::RegionAction {
                region_id,
                action_id,
            },
        );

        assert!(player.favor < favor_before, "the act should cost favor");
        assert!(
            report
                .feedback
                .iter()
                .any(|f| f.level == FeedbackLevel::Success),
            "a successful act reports success"
        );
    }

    #[test]
    fn an_unaffordable_act_warns_and_changes_nothing() {
        let data = GameData::load().unwrap();
        let mut world = WorldState::new(&data);
        let mut player = PlayerState::new(&data.config);
        player.favor = 0;
        let region_id = world.regions[0].id.clone();
        let action_id = data.ordered_region_actions()[0].id.clone();
        let artifacts_before = world.artifacts.len();

        let report = apply(
            &mut world,
            &mut player,
            &data,
            &PlayerAction::RegionAction {
                region_id,
                action_id,
            },
        );

        assert_eq!(player.favor, 0);
        assert_eq!(world.artifacts.len(), artifacts_before);
        assert!(report
            .feedback
            .iter()
            .all(|f| f.level == FeedbackLevel::Warning));
    }
}
