//! The seven divine-tool handlers for `Game` (GDD 5.6): artifacts, pantheon,
//! civilization agendas, myths, magic, and weather. Split from `actions` to
//! keep each file focused; another `impl Game` block over the same fields.

use super::Game;
use crate::data::fill;
use crate::world::{
    adjust_pressure, weather_cost, Artifact, ConsequenceEffect, DelayedConsequence, Myth,
    WeatherEvent,
};

impl Game {
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
        let transfer_instability = balance.transfer_instability;
        if let Some(artifact) = self.world.artifacts.iter_mut().find(|a| a.id == id) {
            artifact.region_id = next_id;
            // Wrenching a bound relic loose unsettles it — the journey adds
            // instability, so moving is a considered act, not a free reposition.
            artifact.instability += transfer_instability;
            let name = artifact.name.clone();
            self.notifications.success(fill(
                &notes.artifact_transferred,
                &[("name", name), ("region", next_name)],
            ));
        }
    }

    pub(super) fn appease_deity(&mut self, id: &str) {
        let amount = self.data.balance.pantheon.appease_amount;
        let cost = self.data.balance.pantheon.appease_cost;
        self.influence_deity(id, -amount, -1.0, cost, true);
    }

    pub(super) fn challenge_deity(&mut self, id: &str) {
        let amount = self.data.balance.pantheon.challenge_amount;
        let cost = self.data.balance.pantheon.challenge_cost;
        self.influence_deity(id, amount, 1.0, cost, false);
    }

    /// Shared appease/challenge logic: move the target's pressure, ripple the
    /// opposite way to its ally (`ripple_sign`) and rival, honouring the
    /// relationship cooldown (GDD 5.6).
    fn influence_deity(
        &mut self,
        id: &str,
        target_delta: f32,
        ripple_sign: f32,
        cost: i64,
        appease: bool,
    ) {
        let notes = self.data.strings.notifications.clone();
        let Some((ally_id, rival_id, name, on_cooldown)) =
            self.world.pantheon.iter().find(|d| d.id == id).map(|d| {
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
            self.notifications
                .warning(fill(&notes.deity_cooldown, &[("deity", name)]));
            return;
        }
        if !self.player.spend(cost, &self.data.balance.player) {
            self.notifications.warning(notes.not_enough_favor);
            return;
        }

        let ripple = self.data.balance.pantheon.ripple;
        let cooldown = self.data.balance.pantheon.cooldown;
        adjust_pressure(&mut self.world.pantheon, id, target_delta);
        adjust_pressure(&mut self.world.pantheon, &ally_id, ripple * ripple_sign);
        adjust_pressure(&mut self.world.pantheon, &rival_id, -ripple * ripple_sign);
        if let Some(deity) = self.world.pantheon.iter_mut().find(|d| d.id == id) {
            deity.cooldown = cooldown;
        }

        let template = if appease {
            &notes.deity_appeased
        } else {
            &notes.deity_challenged
        };
        self.notifications
            .success(fill(template, &[("deity", name)]));
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
        // Shaped weather leaves a delayed mark: a harmful working (net loss of
        // prosperity) scars with flood or famine, a fair one ripens into a later
        // harvest — both unfolding via the consequence queue (GDD 5.6).
        let wb = &self.data.balance.weather;
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
            self.world.pending_consequences.push(DelayedConsequence {
                region_id: region_id.clone(),
                source: pattern.name.clone(),
                delay: wb.aftermath_delay,
                effect,
            });
        }
        self.world
            .weather
            .push(WeatherEvent::from_parts(region_id, &pattern, &intensity));
        self.notifications.success(fill(
            &notes.weather_shaped,
            &[("pattern", pattern.name), ("region", region_name)],
        ));
    }
}
