//! High-level game loop: owns the world, the player, and screen navigation,
//! runs the tick timer, and interprets UI intents.

use crate::data::{fill, ChampionFocus, GameData};
use crate::save::{migrate_save_value, SaveData};
use crate::sim::tick_world;
use crate::ui::{self, Screen, UiAction, UiContext};
use crate::world::{EventKind, PlayerState, WorldState};
use macroquad::prelude::*;
use macroquad_toolkit::events::EventBus;
use macroquad_toolkit::notifications::{
    NotificationAnchor, NotificationManager, NotificationRenderConfig,
};
use macroquad_toolkit::persistence::{
    load_from_slot_with_migration, save_to_slot_with_version, slot_exists,
};
use macroquad_toolkit::prelude::{begin_virtual_ui_frame, dark, end_virtual_ui_frame};

pub struct Game {
    data: GameData,
    world: WorldState,
    player: PlayerState,
    notifications: NotificationManager,
    events: EventBus<UiAction>,
    screen: Screen,
    selected_region: usize,
    save_exists: bool,
    tick_accum: f32,
}

impl Game {
    pub async fn new() -> Self {
        let data = GameData::load().unwrap_or_else(|err| {
            panic!("Mytherra content failed to load: {err}");
        });

        let world = WorldState::new(&data);
        let player = PlayerState::new(&data.config);

        let mut notifications = NotificationManager::new();
        notifications.info(fill(
            &data.strings.notifications.awaken,
            &[("regions", world.regions.len().to_string())],
        ));

        let mut game = Self {
            data,
            world,
            player,
            notifications,
            events: EventBus::new(),
            screen: Screen::Dashboard,
            selected_region: 0,
            save_exists: false,
            tick_accum: 0.0,
        };
        game.refresh_save_state();
        game
    }

    /// Seed a named screen (and some world history) for the screenshot harness.
    pub fn begin_capture_scene(&mut self, scene: &str) {
        self.screen = match scene {
            "regions" => Screen::Regions,
            "heroes" => Screen::Heroes,
            "divine_tools" => Screen::DivineTools,
            "betting" => Screen::Betting,
            "eras" => Screen::Eras,
            _ => Screen::Dashboard,
        };
        // Demo a couple of champions so the heroes screen shows the roster.
        if matches!(self.screen, Screen::Heroes) {
            let ids: Vec<String> = self
                .world
                .heroes
                .iter()
                .filter(|h| h.is_alive)
                .take(2)
                .map(|h| h.id.clone())
                .collect();
            for id in ids {
                self.designate_champion(&id);
            }
        }
        for _ in 0..8 {
            self.run_tick();
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.notifications.update(dt);
        self.handle_input();
        self.advance_tick_timer(dt);

        let actions: Vec<UiAction> = self.events.drain().collect();
        for action in actions {
            self.apply_action(action);
        }
    }

    pub fn draw(&mut self) {
        clear_background(dark::BACKGROUND);

        let virtual_ui = begin_virtual_ui_frame(ui::LOGICAL_WIDTH, ui::LOGICAL_HEIGHT);
        let ctx = UiContext {
            data: &self.data,
            world: &self.world,
            player: &self.player,
            screen: self.screen,
            selected_region: self.selected_region,
            save_exists: self.save_exists,
            seconds_to_tick: (self.data.config.seconds_per_tick - self.tick_accum).max(0.0),
            mouse: virtual_ui.mouse_position(),
        };
        let actions = ui::draw_game_ui(&ctx);
        end_virtual_ui_frame();

        for action in actions {
            self.events.push(action);
        }

        self.notifications
            .draw_with_config(&NotificationRenderConfig {
                anchor: NotificationAnchor::BottomRight,
                ..Default::default()
            });
    }

    fn handle_input(&mut self) {
        if is_key_pressed(KeyCode::S) {
            self.events.push(UiAction::Save);
        }
        if is_key_pressed(KeyCode::L) {
            self.events.push(UiAction::Load);
        }
        if is_key_pressed(KeyCode::N) {
            self.events.push(UiAction::NewWorld);
        }
        if is_key_pressed(KeyCode::Space) {
            self.events.push(UiAction::AdvanceTick);
        }
    }

    fn advance_tick_timer(&mut self, dt: f32) {
        self.tick_accum += dt;
        let interval = self.data.config.seconds_per_tick.max(0.1);
        while self.tick_accum >= interval {
            self.tick_accum -= interval;
            self.run_tick();
        }
    }

    fn run_tick(&mut self) {
        tick_world(&mut self.world, &mut self.player, &self.data);
    }

    fn apply_action(&mut self, action: UiAction) {
        match action {
            UiAction::SelectScreen(screen) => self.screen = screen,
            UiAction::SelectRegion(index) => {
                if index < self.world.regions.len() {
                    self.selected_region = index;
                }
            }
            UiAction::RegionAction(id) => self.apply_region_action(&id),
            UiAction::DesignateChampion(id) => self.designate_champion(&id),
            UiAction::CultivateChampion(id) => self.cultivate_champion(&id),
            UiAction::CycleChampionFocus(id) => self.cycle_champion_focus(&id),
            UiAction::AdvanceTick => {
                self.run_tick();
                self.notifications
                    .info(self.data.strings.notifications.advance_tick.clone());
            }
            UiAction::Save => self.save_game(),
            UiAction::Load => self.load_game(),
            UiAction::NewWorld => self.new_world(),
        }
    }

    fn apply_region_action(&mut self, id: &str) {
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

    fn designate_champion(&mut self, hero_id: &str) {
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

    fn cultivate_champion(&mut self, hero_id: &str) {
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

    fn cycle_champion_focus(&mut self, hero_id: &str) {
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

    fn hero_name(&self, hero_id: &str) -> String {
        self.world
            .heroes
            .iter()
            .find(|h| h.id == hero_id)
            .map(|h| h.name.clone())
            .unwrap_or_else(|| hero_id.to_owned())
    }

    fn new_world(&mut self) {
        self.world = WorldState::new(&self.data);
        self.player = PlayerState::new(&self.data.config);
        self.selected_region = 0;
        self.tick_accum = 0.0;
        self.notifications
            .info(self.data.strings.notifications.new_world.clone());
    }

    fn save_game(&mut self) {
        let save = SaveData::new(&self.world, &self.player, &self.data.config.version);
        let notes = self.data.strings.notifications.clone();
        match save_to_slot_with_version(
            &self.data.config.game_name,
            &self.data.config.save_slot,
            &save,
            &self.data.config.version,
        ) {
            Ok(()) => {
                self.notifications.success(notes.world_saved);
                self.refresh_save_state();
            }
            Err(err) => self
                .notifications
                .danger(fill(&notes.save_failed, &[("error", err)])),
        }
    }

    fn load_game(&mut self) {
        let loaded: Result<SaveData, String> = load_from_slot_with_migration(
            &self.data.config.game_name,
            &self.data.config.save_slot,
            &self.data.config.version,
            |version, value| migrate_save_value(version, value, &self.data.config),
        );
        let notes = self.data.strings.notifications.clone();
        match loaded {
            Ok(save) => {
                self.world = save.world;
                self.player = save.player;
                self.selected_region = 0;
                self.tick_accum = 0.0;
                self.notifications.success(notes.world_restored);
                self.refresh_save_state();
            }
            Err(err) => self
                .notifications
                .warning(fill(&notes.load_failed, &[("error", err)])),
        }
    }

    fn refresh_save_state(&mut self) {
        self.save_exists = slot_exists(&self.data.config.game_name, &self.data.config.save_slot);
    }
}
