//! Immediate-mode UI: screen routing, shared context, and the intent enum.
//!
//! The UI is a pure view layer (RustGames convention): it reads state and
//! returns `UiAction` intents; it never mutates world or player state directly.
//! `Game::apply_action` interprets the intents.

mod betting;
mod dashboard;
mod heroes;
mod placeholder;
mod regions;
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
    Regions,
    Heroes,
    DivineTools,
    Betting,
    Eras,
}

impl Screen {
    pub const ALL: [Screen; 6] = [
        Screen::Dashboard,
        Screen::Regions,
        Screen::Heroes,
        Screen::DivineTools,
        Screen::Betting,
        Screen::Eras,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Screen::Dashboard => "Dashboard",
            Screen::Regions => "Regions",
            Screen::Heroes => "Heroes",
            Screen::DivineTools => "Divine Tools",
            Screen::Betting => "Observatory",
            Screen::Eras => "Eras",
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
    pub mouse: Vec2,
}

/// Draw a whole frame and collect the intents it produced.
pub fn draw_game_ui(ctx: &UiContext<'_>) -> Vec<UiAction> {
    let mut actions = Vec::new();

    shell::draw_header(ctx);
    shell::draw_nav(ctx, &mut actions);

    let placeholders = &ctx.data.strings.placeholders;
    match ctx.screen {
        Screen::Dashboard => dashboard::draw(ctx, &mut actions),
        Screen::Regions => regions::draw(ctx, &mut actions),
        Screen::Heroes => heroes::draw(ctx, &mut actions),
        Screen::DivineTools => placeholder::draw(ctx, &placeholders.divine_tools),
        Screen::Betting => betting::draw(ctx, &mut actions),
        Screen::Eras => placeholder::draw(ctx, &placeholders.eras),
    }

    shell::draw_footer(ctx);
    actions
}

/// The rectangle screens draw their content into, below header + nav.
pub(crate) fn content_rect() -> macroquad::prelude::Rect {
    macroquad::prelude::Rect::new(18.0, 138.0, LOGICAL_WIDTH - 36.0, 520.0)
}
