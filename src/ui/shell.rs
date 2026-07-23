//! Persistent chrome around every screen: header, nav tabs, footer.

use crate::data::fill;
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

    let ui = &ctx.data.strings.ui;
    let y = rect.y + 16.0;
    let mut x = rect.right() - 18.0;
    let (tick_label, tick_fill) = if ctx.paused {
        (ui.tick_paused.clone(), TICK_PAUSED_FILL)
    } else {
        (next_tick_label(ctx), TICK_FILL)
    };
    x = badge_right(x, y, 118.0, &tick_label, tick_fill);
    x = badge_right(
        x,
        y,
        96.0,
        &fill(&ui.level_badge, &[("level", ctx.player.level.to_string())]),
        LEVEL_FILL,
    );
    x = badge_right(
        x,
        y,
        132.0,
        &fill(&ui.favor_badge, &[("favor", ctx.player.favor.to_string())]),
        FAVOR_FILL,
    );
    badge_right(
        x,
        y,
        118.0,
        &fill(&ui.year_badge, &[("year", ctx.world.year.to_string())]),
        YEAR_FILL,
    );
}

pub fn draw_nav(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let rect = Rect::new(18.0, 84.0, LOGICAL_WIDTH - 36.0, 44.0);
    // Only the screens the deity's Standing has revealed appear in the nav — a
    // fledgling Watcher sees a handful; an Elder sees them all (GDD 5.9).
    let revealed: Vec<Screen> = Screen::ALL
        .iter()
        .copied()
        .filter(|s| s.is_revealed(ctx.standing))
        .collect();
    let labels: Vec<&str> = revealed.iter().map(|s| s.label()).collect();
    let active = revealed.iter().position(|s| *s == ctx.screen).unwrap_or(0);
    if let Some(index) = nav_tabs(rect, &labels, active, ctx.mouse) {
        actions.push(UiAction::SelectScreen(revealed[index]));
    }
}

pub fn draw_footer(ctx: &UiContext<'_>) {
    let rect = Rect::new(18.0, 664.0, LOGICAL_WIDTH - 36.0, 40.0);
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.055, 0.06, 0.075, 0.96))
            .with_border(1.0, Color::new(0.38, 0.45, 0.58, 0.45)),
    );
    let hint = fill(
        &ctx.data.strings.ui.footer_hint,
        &[("regions", ctx.world.regions.len().to_string())],
    );
    draw_ui_text_ex(
        &hint,
        rect.x + 16.0,
        rect.y + 26.0,
        TextStyle::new(15.0, dark::TEXT_DIM).params(),
    );
}

fn next_tick_label(ctx: &UiContext<'_>) -> String {
    let seconds = format!("{:>2}", ctx.seconds_to_tick.max(0.0).ceil() as i64);
    fill(&ctx.data.strings.ui.tick_badge, &[("seconds", seconds)])
}

/// Draw a right-anchored badge, returning the new right edge (left of it).
fn badge_right(right: f32, y: f32, w: f32, label: &str, fill_color: Color) -> f32 {
    let rect = Rect::new(right - w, y, w, 28.0);
    draw_badge(rect, label, fill_color, dark::TEXT);
    rect.x - 8.0
}

const YEAR_FILL: Color = Color::new(0.18, 0.24, 0.32, 1.0);
const FAVOR_FILL: Color = Color::new(0.20, 0.28, 0.20, 1.0);
const LEVEL_FILL: Color = Color::new(0.22, 0.19, 0.30, 1.0);
const TICK_FILL: Color = Color::new(0.24, 0.22, 0.16, 1.0);
const TICK_PAUSED_FILL: Color = Color::new(0.30, 0.17, 0.15, 1.0);
