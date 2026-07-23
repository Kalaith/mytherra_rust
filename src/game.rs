//! High-level game loop: owns the world, the player, and screen navigation,
//! runs the tick timer, and interprets UI intents.

mod achievements;
mod capture;
mod command;
mod online;

use crate::data::{fill, ArtifactFocus, GameData};
use crate::sim::tick_world;
use crate::ui::{self, Screen, UiAction, UiContext};
use crate::world::{PlayerState, WorldState};
use macroquad::prelude::*;
use macroquad_toolkit::events::EventBus;
use macroquad_toolkit::notifications::{
    NotificationAnchor, NotificationManager, NotificationRenderConfig,
};
use macroquad_toolkit::prelude::{begin_virtual_ui_frame, dark, end_virtual_ui_frame};
use mytherra_protocol::{project, PlayerAction, Standing, Tier, WorldView};
use online::OnlineSession;

pub struct Game {
    data: GameData,
    world: WorldState,
    player: PlayerState,
    notifications: NotificationManager,
    events: EventBus<UiAction>,
    screen: Screen,
    selected_region: usize,
    /// Settlement id whose detail is open in the Regions town browser (transient
    /// UI state, not persisted); `None` shows the ordinary region detail.
    selected_town: Option<String>,
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
    omens_page: usize,
    /// Eras chronicle page (0-based); clamped by the view as ages accrue.
    eras_page: usize,
    /// Auto-tick cadence (index into `balance.settings.tick_speed_presets`).
    tick_speed_index: usize,
    /// Whether automatic world ticking is paused (Settings, GDD 10).
    paused: bool,
    /// Set when the player chooses Exit; the main loop ends on the next frame.
    quit_requested: bool,
    /// The live connection to the authority server when playing online (GDD 7.1)
    /// — `None` on the title menu and under the headless capture fixture, `Some`
    /// while connected. Interactive play is always online; there is no
    /// local-world mode.
    online: Option<OnlineSession>,
    /// The deity's Standing — what it may see, do, and bet on (GDD 5.9). Online,
    /// it is adopted from the server's `PlayerView`; under the capture fixture it
    /// is derived from the player's level. Gates the nav and every command.
    standing: Standing,
    /// The Standing-filtered projection of the world the UI renders from (§7.7).
    /// Online it is the `WorldView` the server sends; under the capture fixture
    /// it is projected from the local world when it changes (`view_dirty`). Either
    /// way the UI renders from the same shape.
    view: WorldView,
    view_dirty: bool,
}

/// Rasterize every glyph the UI can draw, at every size it uses, once at
/// startup. Macroquad grows the font's glyph atlas lazily, and a growth
/// triggered mid-session — when some interaction first renders a new glyph or
/// size — can corrupt already-cached glyphs, leaving the entire interface in
/// unreadable garbled text. Forcing all atlas growth up front keeps it stable
/// for the whole session.
fn prewarm_font_atlas() {
    use macroquad_toolkit::ui::{default_ui_font, ensure_default_ui_font};
    let _ = ensure_default_ui_font();
    let Some(font) = default_ui_font() else {
        return;
    };
    // Printable ASCII plus the only two non-ASCII marks the UI draws (the
    // middle dot and em dash used throughout the copy).
    let mut chars: Vec<char> = (0x20u32..=0x7E).filter_map(char::from_u32).collect();
    chars.push('\u{00B7}');
    chars.push('\u{2014}');
    // Every logical font size the UI (12-28) and toolkit widgets render, with
    // margin, so no draw ever grows the atlas again.
    for size in 8u16..=40 {
        font.populate_font_cache(&chars, size);
    }
}

impl Game {
    pub async fn new() -> Self {
        prewarm_font_atlas();

        let data = GameData::load().unwrap_or_else(|err| {
            panic!("Mytherra content failed to load: {err}");
        });

        let world = WorldState::new(&data);
        let player = PlayerState::new(&data.config);

        // The world greets the player when they enter it, not on the title menu.
        let notifications = NotificationManager::new();

        // Start at the tick-speed preset matching the configured default.
        let tick_speed_index = data
            .balance
            .settings
            .tick_speed_presets
            .iter()
            .position(|s| (*s - data.config.seconds_per_tick).abs() < f32::EPSILON)
            .unwrap_or(0);

        let tier = Tier::for_level(player.level, &data.balance.player.tier_unlock_levels);
        let standing = data.tiers.standing(tier);
        let (view, _) = project(&world, &player, &standing, &data);

        let mut game = Self {
            data,
            world,
            player,
            notifications,
            events: EventBus::new(),
            screen: Screen::Title,
            selected_region: 0,
            selected_town: None,
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
            omens_page: 0,
            eras_page: 0,
            hero_filter: 0,
            tick_speed_index,
            paused: false,
            quit_requested: false,
            online: None,
            standing,
            view,
            view_dirty: false,
        };
        game.sync_achievements();
        game
    }

    pub fn update(&mut self, dt: f32) {
        self.notifications.update(dt);
        self.handle_input();

        // Under the capture fixture (the only offline path) the local world
        // advances on its own timer; online, the server owns the tick (§7.1).
        if !self.is_online() {
            self.advance_tick_timer(dt);
        }

        let actions: Vec<UiAction> = self.events.drain().collect();
        for action in actions {
            self.apply_action(action);
        }

        if self.is_online() {
            // Poll the connection: adopt any freshly-fetched projection and
            // surface completed action reports.
            self.update_online(dt);
        } else {
            self.check_achievements();
            self.refresh_standing();
            // Rebuild the projection once, after everything that could have
            // changed the local world this frame (ticks, actions, an ascension).
            if self.view_dirty {
                self.refresh_view();
                self.view_dirty = false;
            }
        }
    }

    /// Rebuild the Standing-filtered view the UI renders from (§7.7) by
    /// projecting the local world — the capture fixture's path. Online, the view
    /// is adopted wholesale from the server instead (see `online`).
    fn refresh_view(&mut self) {
        let (view, _) = project(&self.world, &self.player, &self.standing, &self.data);
        self.view = view;
    }

    /// Recompute Standing from the player's level, announcing an ascension when
    /// it crosses into a higher tier. Called each update during play.
    fn refresh_standing(&mut self) {
        let tier = Tier::for_level(
            self.player.level,
            &self.data.balance.player.tier_unlock_levels,
        );
        if tier.rank() == self.standing.tier {
            return;
        }
        if tier.rank() > self.standing.tier {
            self.notifications.success(fill(
                &self.data.strings.notifications.ascension,
                &[("tier", tier.label().to_owned())],
            ));
        }
        self.standing = self.data.tiers.standing(tier);
        // A new tier reveals more of the world — reproject next.
        self.view_dirty = true;
    }

    /// Reconcile the player's saved unlock state with the current achievement
    /// definitions (call after a new world or a load).
    fn sync_achievements(&mut self) {
        self.player
            .achievements
            .sync_definitions(self.data.achievements.clone());
    }

    /// Unlock any newly-earned achievements and toast them.
    fn check_achievements(&mut self) {
        let unlocked = achievements::check(&self.world, &mut self.player, &self.data);
        let xp = self.data.balance.player.achievement_experience;
        for name in unlocked {
            self.notifications.success(fill(
                &self.data.strings.notifications.achievement_unlocked,
                &[("name", name), ("xp", xp.to_string())],
            ));
        }
    }

    pub fn draw(&mut self) {
        clear_background(dark::BACKGROUND);

        let virtual_ui = begin_virtual_ui_frame(ui::LOGICAL_WIDTH, ui::LOGICAL_HEIGHT);
        let ctx = UiContext {
            data: &self.data,
            world: &self.view,
            player: &self.player,
            standing: &self.standing,
            screen: self.screen,
            selected_region: self.selected_region,
            selected_town: self.selected_town.as_deref(),
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
            omens_page: self.omens_page,
            eras_page: self.eras_page,
            hero_filter: self.hero_filter,
            tick_speed_index: self.tick_speed_index,
            paused: self.paused,
            online: self.is_online(),
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
        // Online play is driven entirely through the UI — the shared world is
        // never save/load/tick-stepped from the keyboard. The title menu and the
        // headless capture fixture take no input either.
        if self.screen == Screen::Title || self.is_online() {
            return;
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
        // The world holds still behind the title menu; it only lives once played.
        if self.paused || self.screen == Screen::Title {
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
        let era_before = self.world.era.number;
        tick_world(&mut self.world, &mut self.player, &self.data);
        self.view_dirty = true;
        // An age turning is the world's most consequential event — surface it
        // proactively rather than leaving it to be found in the chronicle.
        if self.world.era.number > era_before {
            self.notifications.info(fill(
                &self.data.strings.notifications.era_dawns,
                &[("era", self.world.era.name.clone())],
            ));
        }
    }

    fn apply_action(&mut self, action: UiAction) {
        match action {
            UiAction::SelectScreen(screen) => self.screen = screen,
            UiAction::SelectRegion(index) => {
                if index < self.world.regions.len() {
                    self.selected_region = index;
                    // A new region's holdings differ; close any open town detail.
                    self.selected_town = None;
                }
            }
            UiAction::SelectTown(id) => self.selected_town = Some(id),
            UiAction::CloseTown => self.selected_town = None,
            UiAction::SetRegionPage(page) => self.region_page = page,
            UiAction::SetOmensPage(page) => self.omens_page = page,
            UiAction::SetErasPage(page) => self.eras_page = page,
            UiAction::RegionAction(id) => self.submit(PlayerAction::RegionAction {
                region_id: self.selected_region_id(),
                action_id: id,
            }),
            UiAction::DesignateChampion(id) => {
                self.submit(PlayerAction::DesignateChampion { hero_id: id })
            }
            UiAction::CultivateChampion(id) => {
                self.submit(PlayerAction::CultivateChampion { hero_id: id })
            }
            UiAction::CycleChampionFocus(id) => {
                // The client cycles; the wire command carries the concrete focus.
                if let Some(focus) = self.next_champion_focus(&id) {
                    self.submit(PlayerAction::SetChampionFocus { hero_id: id, focus });
                }
            }
            UiAction::PlaceBet(id) => self.submit(PlayerAction::PlaceBet {
                event_id: id,
                confidence_index: self.bet_confidence,
                stake_index: self.bet_stake_index,
            }),
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
            UiAction::CreateArtifact => self.submit(PlayerAction::CreateArtifact {
                region_id: self.selected_region_id(),
                focus: self.create_focus,
            }),
            UiAction::EmpowerArtifact(id) => {
                self.submit(PlayerAction::EmpowerArtifact { artifact_id: id })
            }
            UiAction::StabilizeArtifact(id) => {
                self.submit(PlayerAction::StabilizeArtifact { artifact_id: id })
            }
            UiAction::TransferArtifact(id) => {
                // The client resolves the next region; the command names it.
                if let Some(to_region_id) = self.next_region_for_artifact(&id) {
                    self.submit(PlayerAction::TransferArtifact {
                        artifact_id: id,
                        to_region_id,
                    });
                }
            }
            UiAction::ShapeWeather => self.submit(PlayerAction::ShapeWeather {
                region_id: self.selected_region_id(),
                pattern_index: self.weather_pattern,
                intensity_index: self.weather_intensity,
            }),
            UiAction::CycleWeatherPattern => {
                self.weather_pattern =
                    (self.weather_pattern + 1) % self.data.weather_patterns.len();
            }
            UiAction::CycleWeatherIntensity => {
                self.weather_intensity =
                    (self.weather_intensity + 1) % self.data.weather_intensities.len();
            }
            UiAction::ResearchMagic(id) => self.submit(PlayerAction::ResearchMagic { path_id: id }),
            UiAction::PromoteMyth(id) => {
                self.submit(PlayerAction::PromoteMyth { candidate_id: id })
            }
            UiAction::AdvanceAgenda(index) => self.submit(PlayerAction::AdvanceAgenda {
                region_id: self.selected_region_id(),
                agenda_index: index,
            }),
            UiAction::AppeaseDeity(id) => self.submit(PlayerAction::AppeaseDeity { deity_id: id }),
            UiAction::ChallengeDeity(id) => {
                self.submit(PlayerAction::ChallengeDeity { deity_id: id })
            }
            UiAction::AdvanceTick => {
                // The shared world turns on the server's schedule (§7.1) — a
                // player never steps it. (The capture fixture drives ticks
                // directly, not through this intent.)
                if !self.is_online() {
                    self.run_tick();
                    self.notifications
                        .info(self.data.strings.notifications.advance_tick.clone());
                }
            }
            UiAction::EnterWorld => self.go_online(),
            UiAction::ReturnToMenu => self.go_offline(),
            UiAction::ExitGame => self.quit_requested = true,
        }
    }

    /// Whether the player has chosen to exit; the main loop stops when set.
    pub fn quit_requested(&self) -> bool {
        self.quit_requested
    }
}
