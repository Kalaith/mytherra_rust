//! High-level game loop: owns the world, the player, and screen navigation,
//! runs the tick timer, and interprets UI intents.

mod actions;

use crate::data::{fill, ArtifactFocus, GameData};
use crate::save::{migrate_save_value, SaveData};
use crate::sim::tick_world;
use crate::ui::{self, Screen, UiAction, UiContext};
use crate::world::{PlayerState, WorldState};
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
    /// Betting selectors (transient UI state, not persisted).
    bet_confidence: usize,
    bet_stake_index: usize,
    /// Divine-tools UI state.
    divine_tab: usize,
    create_focus: ArtifactFocus,
    weather_pattern: usize,
    weather_intensity: usize,
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
            bet_confidence: 1,
            bet_stake_index: 0,
            divine_tab: 0,
            create_focus: ArtifactFocus::Protection,
            weather_pattern: 0,
            weather_intensity: 0,
        };
        game.refresh_save_state();
        game
    }

    /// Seed a named screen (and some world history) for the screenshot harness.
    pub fn begin_capture_scene(&mut self, scene: &str) {
        self.screen = match scene {
            "regions" => Screen::Regions,
            "heroes" => Screen::Heroes,
            "divine_tools" | "artifacts" | "omens" | "weather" | "magic" | "myths" => {
                Screen::DivineTools
            }
            "betting" => Screen::Betting,
            "eras" => Screen::Eras,
            _ => Screen::Dashboard,
        };
        self.divine_tab = match scene {
            "weather" => 1,
            "omens" => 2,
            "magic" => 3,
            "myths" => 4,
            _ => 0,
        };
        if scene == "weather" {
            self.weather_intensity = 2;
            self.shape_weather();
            self.selected_region = 1;
            self.weather_pattern = 2;
            self.shape_weather();
            self.selected_region = 0;
            self.weather_pattern = 0;
            self.weather_intensity = 0;
        }
        if scene == "magic" {
            for _ in 0..4 {
                self.research_magic("restoration");
            }
            for _ in 0..45 {
                self.run_tick();
            }
        }
        if scene == "myths" {
            for _ in 0..2 {
                self.run_tick();
            }
            let ids: Vec<String> = self
                .world
                .myth_candidates
                .iter()
                .take(2)
                .map(|c| c.id.clone())
                .collect();
            for id in ids {
                self.promote_myth(&id);
            }
            for _ in 0..6 {
                self.run_tick();
            }
        }
        match self.screen {
            // Demo a couple of champions so the heroes screen shows the roster.
            Screen::Heroes => {
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
                for _ in 0..8 {
                    self.run_tick();
                }
            }
            // Demo a couple of wagers so the Observatory shows events and bets.
            Screen::Betting => {
                for _ in 0..3 {
                    self.run_tick();
                }
                let ids: Vec<String> = self
                    .world
                    .speculations
                    .iter()
                    .filter(|e| e.is_active())
                    .take(2)
                    .map(|e| e.id.clone())
                    .collect();
                for id in ids {
                    self.place_bet(&id);
                }
                for _ in 0..12 {
                    self.run_tick();
                }
            }
            _ => {
                for _ in 0..5 {
                    self.run_tick();
                }
            }
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
            bet_confidence: self.bet_confidence,
            bet_stake_index: self.bet_stake_index,
            divine_tab: self.divine_tab,
            create_focus: self.create_focus,
            weather_pattern: self.weather_pattern,
            weather_intensity: self.weather_intensity,
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
            UiAction::PlaceBet(id) => self.place_bet(&id),
            UiAction::CycleConfidence => {
                self.bet_confidence = (self.bet_confidence + 1) % self.data.confidence_levels.len();
            }
            UiAction::CycleStake => {
                self.bet_stake_index =
                    (self.bet_stake_index + 1) % self.data.balance.betting.stake_presets.len();
            }
            UiAction::SelectDivineTab(index) => self.divine_tab = index,
            UiAction::CycleArtifactFocus => self.create_focus = self.create_focus.next(),
            UiAction::CreateArtifact => self.create_artifact(),
            UiAction::EmpowerArtifact(id) => self.empower_artifact(&id),
            UiAction::StabilizeArtifact(id) => self.stabilize_artifact(&id),
            UiAction::TransferArtifact(id) => self.transfer_artifact(&id),
            UiAction::ShapeWeather => self.shape_weather(),
            UiAction::CycleWeatherPattern => {
                self.weather_pattern =
                    (self.weather_pattern + 1) % self.data.weather_patterns.len();
            }
            UiAction::CycleWeatherIntensity => {
                self.weather_intensity =
                    (self.weather_intensity + 1) % self.data.weather_intensities.len();
            }
            UiAction::ResearchMagic(id) => self.research_magic(&id),
            UiAction::PromoteMyth(id) => self.promote_myth(&id),
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
