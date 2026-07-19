//! Project-local UI helpers: thin wrappers that pin Mytherra's chrome and stat
//! ramps on top of the toolkit's `VirtualUi`-safe widgets.
//!
//! The interactive widgets take the already-transformed logical `mouse` (raw
//! screen mouse is wrong inside a letterboxed `VirtualUi` frame). The toolkit
//! now owns that hit-testing via `button_rect_enabled_styled_ex_at` /
//! `tab_bar_styled_at`; these wrappers only supply Mytherra's tone/text/tab
//! styling so the look stays consistent.

use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;

/// A clickable button that hit-tests against the logical `mouse` position.
pub fn button(rect: Rect, text: &str, enabled: bool, tone: ButtonTone, mouse: Vec2) -> bool {
    let style = ButtonStyle::from_tone(tone);
    button_rect_enabled_styled_ex_at(
        rect,
        text,
        enabled,
        &style,
        TextStyle::new(17.0, style.text_color),
        ButtonTrigger::Release,
        mouse,
    )
}

/// A one-of-N horizontal tab bar. Returns the clicked index this frame, if any.
pub fn nav_tabs(rect: Rect, labels: &[&str], active: usize, mouse: Vec2) -> Option<usize> {
    tab_bar_styled_at(
        rect,
        labels,
        active,
        TabOrientation::Horizontal,
        &TabStyle::default(),
        mouse,
    )
}

/// A titled content panel: the shared header/divider/border chrome every screen
/// draws its section into. Keeps one definition instead of a copy per screen.
pub fn draw_titled(rect: Rect, title: &str) {
    let style = SurfaceStyle::new(Color::new(0.07, 0.075, 0.095, 0.96))
        .with_border(1.0, Color::new(0.38, 0.45, 0.58, 0.5))
        .with_header(42.0, Color::new(0.1, 0.115, 0.145, 1.0))
        .with_header_divider(1.0, Color::new(0.38, 0.45, 0.58, 0.4));
    draw_surface_with_title(rect, Some(title), &style, TextStyle::new(20.0, dark::TEXT));
}

/// A rising/falling marker for a stat's per-tick change, with a deadzone so
/// slow mean-reversion drift doesn't flicker an arrow every tick. Shared by the
/// region detail and the dashboard so both read the same way.
pub fn trend_marker(delta: f32) -> &'static str {
    if delta > 0.4 {
        "  ^"
    } else if delta < -0.4 {
        "  v"
    } else {
        ""
    }
}

/// Color ramp for a 0-100 stat where higher is better (prosperity, magic).
pub fn good_stat_color(value: f32) -> Color {
    if value >= 65.0 {
        dark::POSITIVE
    } else if value >= 40.0 {
        dark::WARNING
    } else {
        dark::NEGATIVE
    }
}

/// Color ramp for a 0-100 stat where higher is worse (chaos, danger).
pub fn bad_stat_color(value: f32) -> Color {
    if value >= 65.0 {
        dark::NEGATIVE
    } else if value >= 40.0 {
        dark::WARNING
    } else {
        dark::POSITIVE
    }
}
