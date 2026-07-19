//! High-level game loop: owns the world, the player, and screen navigation,
//! runs the tick timer, and interprets UI intents.

mod actions;
mod capture;

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
    /// Event Log kind filter (0 = all, else `EventKind::ALL[n-1]`).
    chronicle_filter: usize,
    /// Event Log page (0-based), reset to the newest page when the filter changes.
    chronicle_page: usize,
    /// Hero roster region filter (0 = all, else `regions[n-1]`).
    hero_filter: usize,
    /// Hero roster page (0-based), reset when the region filter changes.
    hero_page: usize,
    /// Region roster page (0-based); clamped by the view as regions come and go.
    region_page: usize,
    /// Auto-tick cadence (index into `balance.settings.tick_speed_presets`).
    tick_speed_index: usize,
    /// Whether automatic world ticking is paused (Settings, GDD 10).
    paused: bool,
    /// Tick count at the last autosave, so it fires once per interval.
    last_autosave_tick: u64,
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

        // Start at the tick-speed preset matching the configured default.
        let tick_speed_index = data
            .balance
            .settings
            .tick_speed_presets
            .iter()
            .position(|s| (*s - data.config.seconds_per_tick).abs() < f32::EPSILON)
            .unwrap_or(0);

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
            chronicle_filter: 0,
            chronicle_page: 0,
            hero_page: 0,
            region_page: 0,
            hero_filter: 0,
            tick_speed_index,
            paused: false,
            last_autosave_tick: 0,
        };
        game.refresh_save_state();
        game
    }

    pub fn update(&mut self, dt: f32) {
        self.notifications.update(dt);
        self.handle_input();
        self.advance_tick_timer(dt);

        let actions: Vec<UiAction> = self.events.drain().collect();
        for action in actions {
            self.apply_action(action);
        }

        self.maybe_autosave();
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
            seconds_to_tick: (self.tick_interval() - self.tick_accum).max(0.0),
            bet_confidence: self.bet_confidence,
            bet_stake_index: self.bet_stake_index,
            divine_tab: self.divine_tab,
            create_focus: self.create_focus,
            weather_pattern: self.weather_pattern,
            weather_intensity: self.weather_intensity,
            chronicle_filter: self.chronicle_filter,
            chronicle_page: self.chronicle_page,
            hero_page: self.hero_page,
            region_page: self.region_page,
            hero_filter: self.hero_filter,
            tick_speed_index: self.tick_speed_index,
            paused: self.paused,
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

    /// Real seconds between automatic ticks at the selected pacing preset.
    fn tick_interval(&self) -> f32 {
        self.data
            .balance
            .settings
            .tick_speed_presets
            .get(self.tick_speed_index)
            .copied()
            .unwrap_or(self.data.config.seconds_per_tick)
            .max(0.1)
    }

    fn advance_tick_timer(&mut self, dt: f32) {
        if self.paused {
            self.tick_accum = 0.0;
            return;
        }
        self.tick_accum += dt;
        let interval = self.tick_interval();
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
            UiAction::SetRegionPage(page) => self.region_page = page,
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
            UiAction::SetChronicleFilter(index) => {
                self.chronicle_filter = index;
                self.chronicle_page = 0; // jump back to the newest page
            }
            UiAction::SetChroniclePage(page) => self.chronicle_page = page,
            UiAction::SetHeroFilter(index) => {
                self.hero_filter = index;
                self.hero_page = 0; // a new filter starts at the first page
            }
            UiAction::SetHeroPage(page) => self.hero_page = page,
            UiAction::SetTickSpeed(index) => {
                if index < self.data.balance.settings.tick_speed_presets.len() {
                    self.tick_speed_index = index;
                }
            }
            UiAction::TogglePause => self.paused = !self.paused,
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
            UiAction::AdvanceAgenda(index) => self.advance_agenda(index),
            UiAction::AppeaseDeity(id) => self.appease_deity(&id),
            UiAction::ChallengeDeity(id) => self.challenge_deity(&id),
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
        self.last_autosave_tick = 0;
        self.notifications
            .info(self.data.strings.notifications.new_world.clone());
    }

    /// Write the world to its save slot. Shared by manual save and autosave.
    fn write_save(&self) -> Result<(), String> {
        let save = SaveData::new(&self.world, &self.player, &self.data.config.version);
        save_to_slot_with_version(
            &self.data.config.game_name,
            &self.data.config.save_slot,
            &save,
            &self.data.config.version,
        )
    }

    fn save_game(&mut self) {
        let notes = self.data.strings.notifications.clone();
        match self.write_save() {
            Ok(()) => {
                self.notifications.success(notes.world_saved);
                self.refresh_save_state();
            }
            Err(err) => self
                .notifications
                .danger(fill(&notes.save_failed, &[("error", err)])),
        }
    }

    /// Persist the world once every `autosave_every_ticks` ticks of real play.
    /// Lives in the interactive loop only — the capture harness drives
    /// `run_tick` directly, so headless runs never touch the disk.
    fn maybe_autosave(&mut self) {
        let interval = self.data.config.autosave_every_ticks;
        if interval == 0
            || self.world.tick_count == 0
            || self.world.tick_count == self.last_autosave_tick
            || !self.world.tick_count.is_multiple_of(interval)
        {
            return;
        }
        self.last_autosave_tick = self.world.tick_count;
        let notes = self.data.strings.notifications.clone();
        match self.write_save() {
            Ok(()) => {
                self.notifications.info(notes.world_autosaved);
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
                self.last_autosave_tick = self.world.tick_count;
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
