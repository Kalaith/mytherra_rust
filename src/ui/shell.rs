//! Persistent chrome around every screen: header, nav tabs, footer.

use crate::ui::widgets::nav_tabs;
use crate::ui::{Screen, UiAction, UiContext, LOGICAL_WIDTH};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

pub fn draw_header(ctx: &UiContext<'_>) {
    let rect = Rect::new(18.0, 16.0, LOGICAL_WIDTH - 36.0, 60.0);
    let style = SurfaceStyle::new(Color::new(0.08, 0.09, 0.12, 0.96))
        .with_border(1.0, dark::ACCENT)
        .with_top_highlight(2.0, Color::new(0.55, 0.72, 0.95, 0.75));
    draw_surface(rect, &style);

    draw_ui_text_ex(
        &ctx.data.config.display_name,
        rect.x + 18.0,
        rect.y + 38.0,
        TextStyle::new(28.0, dark::TEXT_BRIGHT).params(),
    );

    let mut x = rect.right() - 18.0;
    x = badge_right(x, rect.y + 16.0, 118.0, &next_tick_label(ctx), TICK_FILL);
    x = badge_right(
        x,
        rect.y + 16.0,
        96.0,
        &format!("Lv {}", ctx.player.level),
        LEVEL_FILL,
    );
    x = badge_right(
        x,
        rect.y + 16.0,
        132.0,
        &format!("Favor {}", ctx.player.favor),
        FAVOR_FILL,
    );
    badge_right(
        x,
        rect.y + 16.0,
        118.0,
        &format!("Year {}", ctx.world.year),
        YEAR_FILL,
    );
}

pub fn draw_nav(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let rect = Rect::new(18.0, 84.0, LOGICAL_WIDTH - 36.0, 44.0);
    let labels: Vec<&str> = Screen::ALL.iter().map(|s| s.label()).collect();
    let active = Screen::ALL
        .iter()
        .position(|s| *s == ctx.screen)
        .unwrap_or(0);
    if let Some(index) = nav_tabs(rect, &labels, active, ctx.mouse) {
        actions.push(UiAction::SelectScreen(Screen::ALL[index]));
    }
}

pub fn draw_footer(ctx: &UiContext<'_>) {
    let rect = Rect::new(18.0, 664.0, LOGICAL_WIDTH - 36.0, 40.0);
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.055, 0.06, 0.075, 0.96))
            .with_border(1.0, Color::new(0.38, 0.45, 0.58, 0.45)),
    );
    let hint = format!(
        "The world advances on its own. {} regions watched  |  S save  L load  N new world  Space advance tick",
        ctx.world.regions.len()
    );
    draw_ui_text_ex(
        &hint,
        rect.x + 16.0,
        rect.y + 26.0,
        TextStyle::new(15.0, dark::TEXT_DIM).params(),
    );
}

fn next_tick_label(ctx: &UiContext<'_>) -> String {
    format!("Tick {:>2.0}s", ctx.seconds_to_tick.max(0.0).ceil())
}

/// Draw a right-anchored badge, returning the new right edge (left of it).
fn badge_right(right: f32, y: f32, w: f32, label: &str, fill: Color) -> f32 {
    let rect = Rect::new(right - w, y, w, 28.0);
    draw_badge(rect, label, fill, dark::TEXT);
    rect.x - 8.0
}

const YEAR_FILL: Color = Color::new(0.18, 0.24, 0.32, 1.0);
const FAVOR_FILL: Color = Color::new(0.20, 0.28, 0.20, 1.0);
const LEVEL_FILL: Color = Color::new(0.22, 0.19, 0.30, 1.0);
const TICK_FILL: Color = Color::new(0.24, 0.22, 0.16, 1.0);
