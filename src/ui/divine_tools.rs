//! Divine Tools: a tabbed screen folding the seven divine tools into one
//! destination (GDD 10). Artifacts and Omens are implemented; the rest show a
//! placeholder until their iteration lands.

mod artifacts;
mod omens;
mod weather;

use crate::ui::widgets::nav_tabs;
use crate::ui::{content_rect, UiAction, UiContext};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

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
}

pub fn draw(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let area = content_rect();
    let tabbar = Rect::new(area.x, area.y, area.w, 40.0);
    let labels: Vec<&str> = DivineTool::ALL.iter().map(|t| t.label()).collect();
    let active = ctx.divine_tab.min(DivineTool::ALL.len() - 1);
    if let Some(index) = nav_tabs(tabbar, &labels, active, ctx.mouse) {
        actions.push(UiAction::SelectDivineTab(index));
    }

    let body = Rect::new(area.x, area.y + 48.0, area.w, area.h - 48.0);
    match DivineTool::ALL[active] {
        DivineTool::Artifacts => artifacts::draw(ctx, body, actions),
        DivineTool::Weather => weather::draw(ctx, body, actions),
        DivineTool::Omens => omens::draw(ctx, body),
        _ => draw_todo(ctx, body),
    }
}

fn draw_todo(ctx: &UiContext<'_>, rect: Rect) {
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.07, 0.075, 0.095, 0.96))
            .with_border(1.0, Color::new(0.38, 0.45, 0.58, 0.5)),
    );
    draw_ui_text_ex(
        &ctx.data.strings.divine.tool_todo,
        rect.x + 24.0,
        rect.y + 40.0,
        TextStyle::new(16.0, dark::TEXT_DIM).params(),
    );
}

/// Shared titled-surface helper for the divine-tool sub-screens.
pub(super) fn draw_panel(rect: Rect, title: &str) {
    let style = SurfaceStyle::new(Color::new(0.07, 0.075, 0.095, 0.96))
        .with_border(1.0, Color::new(0.38, 0.45, 0.58, 0.5))
        .with_header(40.0, Color::new(0.1, 0.115, 0.145, 1.0))
        .with_header_divider(1.0, Color::new(0.38, 0.45, 0.58, 0.4));
    draw_surface_with_title(rect, Some(title), &style, TextStyle::new(19.0, dark::TEXT));
}
