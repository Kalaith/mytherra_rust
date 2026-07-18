//! Immediate-mode UI: screen routing, shared context, and the intent enum.
//!
//! The UI is a pure view layer (RustGames convention): it reads state and
//! returns `UiAction` intents; it never mutates world or player state directly.
//! `Game::apply_action` interprets the intents.

mod betting;
mod chronicle;
mod dashboard;
mod divine_tools;
mod eras;
mod heroes;
mod regions;
mod settings;
mod shell;
mod widgets;

use crate::data::GameData;
use crate::world::{PlayerState, WorldState};
use macroquad::prelude::Vec2;

pub const LOGICAL_WIDTH: f32 = 1280.0;
pub const LOGICAL_HEIGHT: f32 = 720.0;

/// Top-level navigable screens (GDD 10). The seven divine tools fold into a
/// single tabbed screen rather than separate destinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Dashboard,
    Chronicle,
    Regions,
    Heroes,
    DivineTools,
    Betting,
    Eras,
    Settings,
}

impl Screen {
    pub const ALL: [Screen; 8] = [
        Screen::Dashboard,
        Screen::Chronicle,
        Screen::Regions,
        Screen::Heroes,
        Screen::DivineTools,
        Screen::Betting,
        Screen::Eras,
        Screen::Settings,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Screen::Dashboard => "Dashboard",
            Screen::Chronicle => "Event Log",
            Screen::Regions => "Regions",
            Screen::Heroes => "Heroes",
            Screen::DivineTools => "Divine Tools",
            Screen::Betting => "Observatory",
            Screen::Eras => "Eras",
            Screen::Settings => "Settings",
        }
    }
}

/// Intents emitted by the view layer for `Game` to interpret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    SelectScreen(Screen),
    SelectRegion(usize),
    /// Apply a region action (by id) to the currently selected region.
    RegionAction(String),
    /// Designate the given hero (by id) as a champion.
    DesignateChampion(String),
    /// Cultivate the champion bonded to the given hero id.
    CultivateChampion(String),
    /// Cycle the cultivation focus of the champion bonded to the given hero id.
    CycleChampionFocus(String),
    /// Place a bet on the given speculation event id (using current selectors).
    PlaceBet(String),
    /// Cycle the selected confidence tier for the next bet.
    CycleConfidence,
    /// Cycle the selected stake preset for the next bet.
    CycleStake,
    /// Set the Event Log kind filter (0 = all, else `EventKind::ALL[n-1]`).
    SetChronicleFilter(usize),
    /// Select the auto-tick cadence by preset index (Settings, GDD 10).
    SetTickSpeed(usize),
    /// Toggle automatic world ticking on/off (Settings, GDD 10).
    TogglePause,
    /// Select a divine-tool sub-tab by index.
    SelectDivineTab(usize),
    /// Cycle the focus of the next artifact to be forged.
    CycleArtifactFocus,
    /// Forge a new artifact in the selected region.
    CreateArtifact,
    /// Empower the artifact with the given id.
    EmpowerArtifact(String),
    /// Stabilize the artifact with the given id.
    StabilizeArtifact(String),
    /// Move the artifact with the given id to another region.
    TransferArtifact(String),
    /// Shape weather over the selected region with the current selectors.
    ShapeWeather,
    /// Cycle the selected weather pattern.
    CycleWeatherPattern,
    /// Cycle the selected weather intensity.
    CycleWeatherIntensity,
    /// Pour favor into researching the given magic path id.
    ResearchMagic(String),
    /// Promote the myth candidate with the given id into a living myth.
    PromoteMyth(String),
    /// Advance the agenda at the given index in the selected region.
    AdvanceAgenda(usize),
    /// Appease the pantheon deity with the given id.
    AppeaseDeity(String),
    /// Challenge the pantheon deity with the given id.
    ChallengeDeity(String),
    AdvanceTick,
    Save,
    Load,
    NewWorld,
}

/// Everything the view layer needs to render a frame.
pub struct UiContext<'a> {
    pub data: &'a GameData,
    pub world: &'a WorldState,
    pub player: &'a PlayerState,
    pub screen: Screen,
    pub selected_region: usize,
    pub save_exists: bool,
    pub seconds_to_tick: f32,
    /// Index into `data.confidence_levels` for the next bet.
    pub bet_confidence: usize,
    /// Index into `balance.betting.stake_presets` for the next bet.
    pub bet_stake_index: usize,
    /// Selected divine-tool sub-tab index.
    pub divine_tab: usize,
    /// Focus of the next artifact to be forged.
    pub create_focus: crate::data::ArtifactFocus,
    /// Selected weather pattern / intensity indices.
    pub weather_pattern: usize,
    pub weather_intensity: usize,
    /// Event Log kind filter (0 = all, else `EventKind::ALL[n-1]`).
    pub chronicle_filter: usize,
    /// Selected auto-tick cadence (index into `balance.settings.tick_speed_presets`).
    pub tick_speed_index: usize,
    /// Whether automatic world ticking is paused.
    pub paused: bool,
    pub mouse: Vec2,
}

/// Draw a whole frame and collect the intents it produced.
pub fn draw_game_ui(ctx: &UiContext<'_>) -> Vec<UiAction> {
    let mut actions = Vec::new();

    shell::draw_header(ctx);
    shell::draw_nav(ctx, &mut actions);

    match ctx.screen {
        Screen::Dashboard => dashboard::draw(ctx, &mut actions),
        Screen::Chronicle => chronicle::draw(ctx, &mut actions),
        Screen::Regions => regions::draw(ctx, &mut actions),
        Screen::Heroes => heroes::draw(ctx, &mut actions),
        Screen::DivineTools => divine_tools::draw(ctx, &mut actions),
        Screen::Betting => betting::draw(ctx, &mut actions),
        Screen::Eras => eras::draw(ctx),
        Screen::Settings => settings::draw(ctx, &mut actions),
    }

    shell::draw_footer(ctx);
    actions
}

/// The rectangle screens draw their content into, below header + nav.
pub(crate) fn content_rect() -> macroquad::prelude::Rect {
    macroquad::prelude::Rect::new(18.0, 138.0, LOGICAL_WIDTH - 36.0, 520.0)
}
