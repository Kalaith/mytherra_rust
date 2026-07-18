//! UI-intent handlers for `Game` — the verbs the player invokes (region
//! actions, champion cultivation, betting, artifacts). Split from `game.rs` to
//! keep each file focused; this is a second `impl Game` block and reaches the
//! same private fields as the main loop.

use super::Game;
use crate::data::{fill, ChampionFocus};
use crate::world::{quote_event, weather_cost, Artifact, Bet, EventKind, Myth, WeatherEvent};

impl Game {
    pub(super) fn apply_region_action(&mut self, id: &str) {
        let notes = &self.data.strings.notifications;
        let Some(def) = self.data.region_actions.get(id).cloned() else {
            self.notifications
                .warning(fill(&notes.unknown_action, &[("id", id.to_owned())]));
            return;
        };
        let index = self
            .selected_region
            .min(self.world.regions.len().saturating_sub(1));
        let Some(region) = self.world.region(index) else {
            return;
        };
        let cost = region.action_cost(&def, &self.data.balance.region);
        if !self.player.spend(cost, &self.data.balance.player) {
            self.notifications.warning(notes.not_enough_favor.clone());
            return;
        }

        let year = self.world.year;
        let region_name;
        {
            let region = self.world.region_mut(index).expect("index checked above");
            region.apply_action(&def, &self.data.balance.region);
            region_name = region.name.clone();
        }
        let text = &self.data.strings;
        self.world.chronicle.push(
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
        self.notifications.success(fill(
            &text.notifications.action_success,
            &[
                ("action", def.name.clone()),
                ("region", region_name),
                ("cost", cost.to_string()),
            ],
        ));
    }

    pub(super) fn designate_champion(&mut self, hero_id: &str) {
        let notes = self.data.strings.notifications.clone();
        let hero_name = match self
            .world
            .heroes
            .iter()
            .find(|h| h.id == hero_id && h.is_alive)
        {
            Some(hero) => hero.name.clone(),
            None => return,
        };
        let balance = &self.data.balance.champion;
        if self.player.is_champion(hero_id) || self.player.champions.len() >= balance.max_roster {
            self.notifications.warning(notes.champion_designate_failed);
            return;
        }
        if !self
            .player
            .spend(balance.designate_cost, &self.data.balance.player)
        {
            self.notifications.warning(notes.not_enough_favor);
            return;
        }
        self.player
            .designate_champion(hero_id, ChampionFocus::Valor, &self.data.balance.champion);
        self.notifications
            .success(fill(&notes.champion_designated, &[("hero", hero_name)]));
    }

    pub(super) fn cultivate_champion(&mut self, hero_id: &str) {
        let notes = self.data.strings.notifications.clone();
        let Some(cost) = self
            .player
            .champions
            .iter()
            .find(|c| c.hero_id == hero_id)
            .map(|c| c.cultivate_cost(&self.data.balance.champion))
        else {
            return;
        };
        let alive = self
            .world
            .heroes
            .iter()
            .any(|h| h.id == hero_id && h.is_alive);
        if !alive {
            return;
        }
        if !self.player.spend(cost, &self.data.balance.player) {
            self.notifications.warning(notes.not_enough_favor);
            return;
        }
        let gain = self.data.balance.champion.cultivate_bond_gain;
        if let Some(champion) = self.player.champion_mut(hero_id) {
            champion.bond += gain;
            champion.recompute_rank(&self.data.balance.champion);
        }
        let hero_name = self.hero_name(hero_id);
        self.notifications
            .success(fill(&notes.champion_cultivated, &[("hero", hero_name)]));
    }

    pub(super) fn cycle_champion_focus(&mut self, hero_id: &str) {
        let Some(champion) = self.player.champion_mut(hero_id) else {
            return;
        };
        champion.focus = champion.focus.next();
        let focus = champion.focus;
        let hero_name = self.hero_name(hero_id);
        self.notifications.info(fill(
            &self.data.strings.notifications.champion_focus_changed,
            &[("hero", hero_name), ("focus", focus.label().to_owned())],
        ));
    }

    pub(super) fn place_bet(&mut self, event_id: &str) {
        let notes = self.data.strings.notifications.clone();
        let betting = &self.data.balance.betting;
        let stake =
            betting.stake_presets[self.bet_stake_index.min(betting.stake_presets.len() - 1)];
        let confidence = self.data.confidence_levels[self
            .bet_confidence
            .min(self.data.confidence_levels.len() - 1)]
        .clone();

        let Some(idx) = self
            .world
            .speculations
            .iter()
            .position(|e| e.id == event_id && e.is_active())
        else {
            self.notifications.warning(notes.bet_closed);
            return;
        };

        let (quote, target_name, bet_type_name, deadline) = {
            let event = &self.world.speculations[idx];
            let likelihood = event.likelihood(&self.world.heroes, &self.world.regions);
            let quote = quote_event(
                event,
                likelihood,
                &confidence,
                stake,
                &self.data.balance.betting,
            );
            (
                quote,
                event.target_name.clone(),
                event.bet_type_name.clone(),
                event.deadline_year,
            )
        };

        if !self.player.place_stake(stake) {
            self.notifications.warning(notes.bet_unaffordable);
            return;
        }
        // The player joins the "yes" side, shifting the crowd lean for later bets.
        self.world.speculations[idx].crowd_yes += stake as f32;

        self.player.bets.push(Bet {
            event_id: event_id.to_owned(),
            bet_type_name,
            target_name: target_name.clone(),
            confidence_name: confidence.name.clone(),
            stake,
            potential_payout: quote.payout,
            odds: quote.odds,
            placed_year: self.world.year,
            deadline_year: deadline,
            resolved: None,
        });
        self.notifications.success(fill(
            &notes.bet_placed,
            &[("target", target_name), ("stake", stake.to_string())],
        ));
    }

    pub(super) fn create_artifact(&mut self) {
        let notes = self.data.strings.notifications.clone();
        let balance = &self.data.balance.artifact;
        if self.world.artifacts.len() >= balance.max_active {
            self.notifications.warning(notes.artifact_max);
            return;
        }
        if !self
            .player
            .spend(balance.create_cost, &self.data.balance.player)
        {
            self.notifications.warning(notes.not_enough_favor);
            return;
        }
        let index = self
            .selected_region
            .min(self.world.regions.len().saturating_sub(1));
        let region_id = self
            .world
            .regions
            .get(index)
            .map(|r| r.id.clone())
            .unwrap_or_default();

        self.world.artifact_seq += 1;
        let seq = self.world.artifact_seq;
        let focus = self.create_focus;
        let name = fill(
            &self.data.strings.divine.new_artifact_name,
            &[("focus", focus.label().to_owned()), ("n", seq.to_string())],
        );
        self.world.artifacts.push(Artifact {
            id: format!("art-{seq}"),
            name: name.clone(),
            focus,
            power: 1,
            instability: 0.0,
            region_id,
        });
        self.notifications
            .success(fill(&notes.artifact_created, &[("name", name)]));
    }

    pub(super) fn empower_artifact(&mut self, id: &str) {
        let notes = self.data.strings.notifications.clone();
        let Some(cost) = self
            .world
            .artifacts
            .iter()
            .find(|a| a.id == id)
            .map(|a| a.empower_cost(&self.data.balance.artifact))
        else {
            return;
        };
        if !self.player.spend(cost, &self.data.balance.player) {
            self.notifications.warning(notes.not_enough_favor);
            return;
        }
        let gain = self.data.balance.artifact.empower_instability_gain;
        if let Some(artifact) = self.world.artifacts.iter_mut().find(|a| a.id == id) {
            artifact.power += 1;
            artifact.instability += gain;
            let name = artifact.name.clone();
            self.notifications
                .success(fill(&notes.artifact_empowered, &[("name", name)]));
        }
    }

    pub(super) fn stabilize_artifact(&mut self, id: &str) {
        let notes = self.data.strings.notifications.clone();
        let balance = &self.data.balance.artifact;
        if !self.world.artifacts.iter().any(|a| a.id == id) {
            return;
        }
        if !self
            .player
            .spend(balance.stabilize_cost, &self.data.balance.player)
        {
            self.notifications.warning(notes.not_enough_favor);
            return;
        }
        let amount = self.data.balance.artifact.stabilize_amount;
        if let Some(artifact) = self.world.artifacts.iter_mut().find(|a| a.id == id) {
            artifact.instability = (artifact.instability - amount).max(0.0);
            let name = artifact.name.clone();
            self.notifications
                .success(fill(&notes.artifact_stabilized, &[("name", name)]));
        }
    }

    pub(super) fn transfer_artifact(&mut self, id: &str) {
        let notes = self.data.strings.notifications.clone();
        let balance = &self.data.balance.artifact;
        let Some(current) = self
            .world
            .artifacts
            .iter()
            .find(|a| a.id == id)
            .map(|a| a.region_id.clone())
        else {
            return;
        };
        let Some(cur_idx) = self.world.regions.iter().position(|r| r.id == current) else {
            return;
        };
        if self.world.regions.len() < 2 {
            return;
        }
        if !self
            .player
            .spend(balance.transfer_cost, &self.data.balance.player)
        {
            self.notifications.warning(notes.not_enough_favor);
            return;
        }
        let next = &self.world.regions[(cur_idx + 1) % self.world.regions.len()];
        let (next_id, next_name) = (next.id.clone(), next.name.clone());
        if let Some(artifact) = self.world.artifacts.iter_mut().find(|a| a.id == id) {
            artifact.region_id = next_id;
            let name = artifact.name.clone();
            self.notifications.success(fill(
                &notes.artifact_transferred,
                &[("name", name), ("region", next_name)],
            ));
        }
    }

    pub(super) fn advance_agenda(&mut self, agenda_index: usize) {
        let notes = self.data.strings.notifications.clone();
        if agenda_index >= self.data.agendas.len() {
            return;
        }
        let region_index = self
            .selected_region
            .min(self.world.regions.len().saturating_sub(1));
        let Some((region_id, region_name)) = self
            .world
            .regions
            .get(region_index)
            .map(|r| (r.id.clone(), r.name.clone()))
        else {
            return;
        };

        let on_cooldown = self
            .world
            .civilization
            .iter()
            .find(|e| e.region_id == region_id)
            .map(|e| e.cooldown > 0)
            .unwrap_or(true);
        if on_cooldown {
            self.notifications.warning(notes.agenda_cooldown);
            return;
        }

        let cost = self.data.balance.civilization.advance_cost;
        if !self.player.spend(cost, &self.data.balance.player) {
            self.notifications.warning(notes.not_enough_favor);
            return;
        }
        let boost = self.data.balance.civilization.advance_boost;
        let cooldown = self.data.balance.civilization.advance_cooldown;
        if let Some(entry) = self
            .world
            .civilization
            .iter_mut()
            .find(|e| e.region_id == region_id)
        {
            if let Some(value) = entry.boosts.get_mut(agenda_index) {
                *value += boost;
            }
            entry.cooldown = cooldown;
        }
        let agenda_name = self.data.agendas[agenda_index].name.clone();
        self.notifications.success(fill(
            &notes.agenda_advanced,
            &[("agenda", agenda_name), ("region", region_name)],
        ));
    }

    pub(super) fn promote_myth(&mut self, id: &str) {
        let notes = self.data.strings.notifications.clone();
        if self.world.myths.len() >= self.data.balance.myth.cap {
            self.notifications.warning(notes.myth_cap);
            return;
        }
        let Some(pos) = self.world.myth_candidates.iter().position(|c| c.id == id) else {
            return;
        };
        let cost = self.data.balance.myth.promote_cost;
        if !self.player.spend(cost, &self.data.balance.player) {
            self.notifications.warning(notes.not_enough_favor);
            return;
        }
        let cooldown = self.data.balance.myth.echo_cooldown;
        let candidate = self.world.myth_candidates.remove(pos);
        let title = candidate.title.clone();
        self.world
            .myths
            .push(Myth::from_candidate(&candidate, cooldown));
        self.notifications
            .success(fill(&notes.myth_promoted, &[("title", title)]));
    }

    pub(super) fn research_magic(&mut self, id: &str) {
        let notes = self.data.strings.notifications.clone();
        if !self.world.magic_paths.iter().any(|p| p.id == id) {
            return;
        }
        let cost = self.data.balance.magic.research_cost;
        if !self.player.spend(cost, &self.data.balance.player) {
            self.notifications.warning(notes.not_enough_favor);
            return;
        }
        let pgain = self.data.balance.magic.research_progress_gain;
        let egain = self.data.balance.magic.research_evidence_gain;
        let cap = self.data.balance.magic.stat_cap;
        if let Some(path) = self.world.magic_paths.iter_mut().find(|p| p.id == id) {
            path.progress = (path.progress + pgain).min(cap);
            path.evidence = (path.evidence + egain).min(cap);
            path.recompute_state(&self.data.balance.magic);
            let name = path.name.clone();
            self.notifications
                .success(fill(&notes.magic_researched, &[("path", name)]));
        }
    }

    pub(super) fn shape_weather(&mut self) {
        let notes = self.data.strings.notifications.clone();
        if self.world.weather.len() >= self.data.balance.weather.max_active {
            self.notifications.warning(notes.weather_max);
            return;
        }
        let pattern = self.data.weather_patterns[self
            .weather_pattern
            .min(self.data.weather_patterns.len() - 1)]
        .clone();
        let intensity = self.data.weather_intensities[self
            .weather_intensity
            .min(self.data.weather_intensities.len() - 1)]
        .clone();
        let index = self
            .selected_region
            .min(self.world.regions.len().saturating_sub(1));
        let Some((region_id, region_name, cost)) = self.world.regions.get(index).map(|r| {
            let cost = weather_cost(
                self.data.balance.weather.base_cost,
                intensity.cost_mult,
                r.cost_multiplier(&self.data.balance.region),
            );
            (r.id.clone(), r.name.clone(), cost)
        }) else {
            return;
        };
        if !self.player.spend(cost, &self.data.balance.player) {
            self.notifications.warning(notes.not_enough_favor);
            return;
        }
        self.world.weather.push(WeatherEvent {
            region_id,
            pattern_id: pattern.id.clone(),
            pattern_name: pattern.name.clone(),
            intensity_name: intensity.name.clone(),
            magnitude: intensity.magnitude,
            prosperity: pattern.prosperity,
            chaos: pattern.chaos,
            danger: pattern.danger,
            magic: pattern.magic,
        });
        self.notifications.success(fill(
            &notes.weather_shaped,
            &[("pattern", pattern.name), ("region", region_name)],
        ));
    }

    pub(super) fn hero_name(&self, hero_id: &str) -> String {
        self.world
            .heroes
            .iter()
            .find(|h| h.id == hero_id)
            .map(|h| h.name.clone())
            .unwrap_or_else(|| hero_id.to_owned())
    }
}
