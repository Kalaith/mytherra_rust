//! Core player-verb handlers for `Game` — region actions, champion
//! cultivation, and betting. The seven divine tools live in `actions_tools`.
//! Both are further `impl Game` blocks reaching the same private fields.

use super::Game;
use crate::data::{fill, ChampionFocus};
use crate::world::{quote_event, Bet, EventKind};

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

        let (quote, target_name, bet_type_name, deadline, predicate) = {
            let event = &self.world.speculations[idx];
            let era_progress =
                self.world.era.pressure / self.data.balance.era.breaking_threshold.max(1.0);
            let likelihood = event.likelihood(
                &self.world.heroes,
                &self.world.regions,
                &self.world.settlements,
                era_progress,
            );
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
                event.predicate,
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
            predicate,
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

    pub(super) fn hero_name(&self, hero_id: &str) -> String {
        self.world
            .heroes
            .iter()
            .find(|h| h.id == hero_id)
            .map(|h| h.name.clone())
            .unwrap_or_else(|| hero_id.to_owned())
    }
}
