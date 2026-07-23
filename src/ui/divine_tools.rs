//! Divine Tools: a tabbed screen folding all seven divine tools into one
//! destination (GDD 10).

mod artifacts;
mod civilization;
mod magic;
mod myths;
mod omens;
mod pantheon;
mod weather;

use crate::ui::widgets::nav_tabs;
use crate::ui::{content_rect, UiAction, UiContext};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use mytherra_protocol::{ActionVerb, Standing};

/// The seven divine tools, in GDD screen order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DivineTool {
    Artifacts,
    Weather,
    Omens,
    Magic,
    Myths,
    Civilization,
    Pantheon,
}

impl DivineTool {
    pub const ALL: [DivineTool; 7] = [
        DivineTool::Artifacts,
        DivineTool::Weather,
        DivineTool::Omens,
        DivineTool::Magic,
        DivineTool::Myths,
        DivineTool::Civilization,
        DivineTool::Pantheon,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DivineTool::Artifacts => "Artifacts",
            DivineTool::Weather => "Weather",
            DivineTool::Omens => "Omens",
            DivineTool::Magic => "Magic",
            DivineTool::Myths => "Myths",
            DivineTool::Civilization => "Civilization",
            DivineTool::Pantheon => "Pantheon",
        }
    }

    /// The verb a player's Standing must have unlocked for this tool's tab to
    /// appear (GDD 5.9). Omens is a read-only forecast (`None`) — shown whenever
    /// the Divine Tools screen itself is.
    pub fn required_verb(self) -> Option<ActionVerb> {
        match self {
            DivineTool::Artifacts => Some(ActionVerb::Artifact),
            DivineTool::Weather => Some(ActionVerb::Weather),
            DivineTool::Omens => None,
            DivineTool::Magic => Some(ActionVerb::Magic),
            DivineTool::Myths => Some(ActionVerb::Myth),
            DivineTool::Civilization => Some(ActionVerb::Agenda),
            DivineTool::Pantheon => Some(ActionVerb::Pantheon),
        }
    }

    pub fn is_revealed(self, standing: &Standing) -> bool {
        self.required_verb()
            .is_none_or(|verb| standing.can_do(verb))
    }
}

pub fn draw(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let area = content_rect();
    let tabbar = Rect::new(area.x, area.y, area.w, 40.0);
    // Only the tools the deity's Standing has unlocked appear (GDD 5.9): a Shaper
    // sees Artifacts/Omens/Magic/Myths; an Elder also gets Weather/Civilization/
    // Pantheon.
    let revealed: Vec<DivineTool> = DivineTool::ALL
        .iter()
        .copied()
        .filter(|t| t.is_revealed(ctx.standing))
        .collect();
    if revealed.is_empty() {
        return;
    }
    let labels: Vec<&str> = revealed.iter().map(|t| t.label()).collect();
    let active = ctx.divine_tab.min(revealed.len() - 1);
    if let Some(index) = nav_tabs(tabbar, &labels, active, ctx.mouse) {
        actions.push(UiAction::SelectDivineTab(index));
    }

    let body = Rect::new(area.x, area.y + 48.0, area.w, area.h - 48.0);
    match revealed[active] {
        DivineTool::Artifacts => artifacts::draw(ctx, body, actions),
        DivineTool::Weather => weather::draw(ctx, body, actions),
        DivineTool::Omens => omens::draw(ctx, body, actions),
        DivineTool::Magic => magic::draw(ctx, body, actions),
        DivineTool::Myths => myths::draw(ctx, body, actions),
        DivineTool::Civilization => civilization::draw(ctx, body, actions),
        DivineTool::Pantheon => pantheon::draw(ctx, body, actions),
    }
}

/// Shared titled-surface helper for the divine-tool sub-screens.
pub(super) fn draw_panel(rect: Rect, title: &str) {
    let style = SurfaceStyle::new(Color::new(0.07, 0.075, 0.095, 0.96))
        .with_border(1.0, Color::new(0.38, 0.45, 0.58, 0.5))
        .with_header(40.0, Color::new(0.1, 0.115, 0.145, 1.0))
        .with_header_divider(1.0, Color::new(0.38, 0.45, 0.58, 0.4));
    draw_surface_with_title(rect, Some(title), &style, TextStyle::new(19.0, dark::TEXT));
}
