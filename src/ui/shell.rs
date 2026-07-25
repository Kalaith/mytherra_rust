//! Persistent chrome around every screen: header, nav tabs, footer.

use crate::data::fill;
use crate::game::OnlineStatus;
use crate::ui::theme;
use crate::ui::widgets::nav_tabs;
use crate::ui::{Screen, UiAction, UiContext, LOGICAL_WIDTH};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

pub fn draw_header(ctx: &UiContext<'_>) {
    let rect = Rect::new(18.0, 16.0, LOGICAL_WIDTH - 36.0, 60.0);
    let style = SurfaceStyle::new(theme::CHROME)
        .with_border(1.0, theme::BORDER)
        .with_top_highlight(2.0, theme::BLUE_EDGE);
    draw_surface(rect, &style);

    // Brand emblem: a lit blue rune, left of the world's name.
    let cx = rect.x + 34.0;
    let cy = rect.y + rect.h * 0.5;
    draw_circle_lines(cx, cy, 16.0, 2.0, theme::BLUE);
    draw_circle(cx, cy, 5.5, theme::BLUE_EDGE);

    let strings = &ctx.data.strings;
    draw_ui_text_ex(
        &ctx.data.config.display_name,
        rect.x + 62.0,
        rect.y + 30.0,
        TextStyle::new(26.0, dark::TEXT_BRIGHT).params(),
    );
    draw_ui_text_ex(
        &strings.ui.header_tagline,
        rect.x + 62.0,
        rect.y + 49.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );

    // Right-anchored resource chips, laid out right-to-left (mockup chrome).
    let ui = &strings.ui;
    let y = rect.y + 8.0;
    let h = 44.0;
    let mut x = rect.right() - 14.0;

    let (tick_value, tick_dot) = tick_chip(ctx);
    x = theme::chip_right(x, y, 100.0, h, tick_dot, &ui.chip_tick, &tick_value);
    x = theme::chip_right(
        x,
        y,
        92.0,
        h,
        LEVEL_DOT,
        &ui.chip_level,
        &ctx.player.level.to_string(),
    );
    x = theme::chip_right(
        x,
        y,
        118.0,
        h,
        theme::GOLD,
        &ui.chip_favour,
        &ctx.player.favor.to_string(),
    );
    theme::chip_right(
        x,
        y,
        108.0,
        h,
        YEAR_DOT,
        &ui.chip_year,
        &ctx.world.year.to_string(),
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
        &SurfaceStyle::new(theme::CHROME).with_border(1.0, theme::BORDER_SOFT),
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

/// The tick chip's value and status dot: a live link online (green/amber/red),
/// or the local countdown / paused state offline.
fn tick_chip(ctx: &UiContext<'_>) -> (String, Color) {
    let ui = &ctx.data.strings.ui;
    if ctx.online {
        // The world turns on the server's schedule — no local countdown. The dot
        // reflects the live link, so a dropped server shows at a glance rather
        // than a silently frozen world.
        match ctx.online_status {
            Some(OnlineStatus::Reconnecting) => (ui.tick_reconnecting.clone(), TICK_OFFLINE_DOT),
            Some(OnlineStatus::Connecting) => (ui.tick_connecting.clone(), TICK_PENDING_DOT),
            _ => (ui.tick_live.clone(), dark::POSITIVE),
        }
    } else if ctx.paused {
        (ui.tick_paused.clone(), TICK_OFFLINE_DOT)
    } else {
        let seconds = format!("{:>2}", ctx.seconds_to_tick.max(0.0).ceil() as i64);
        (fill(&ui.tick_badge, &[("seconds", seconds)]), theme::BLUE)
    }
}

/// Slate — the passing years.
const YEAR_DOT: Color = Color::new(0.45, 0.58, 0.78, 1.0);
/// Violet — the deity's own standing.
const LEVEL_DOT: Color = Color::new(0.62, 0.48, 0.86, 1.0);
/// Amber — reaching out / handshaking.
const TICK_PENDING_DOT: Color = Color::new(0.86, 0.70, 0.30, 1.0);
/// Red — the server is unreachable and the client is retrying.
const TICK_OFFLINE_DOT: Color = Color::new(0.86, 0.34, 0.34, 1.0);
