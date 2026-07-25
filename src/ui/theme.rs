//! Mytherra's project-local visual theme: a blue-forward palette and the small
//! surface / tab / chip / stat helpers that pin the game's look on top of the
//! shared toolkit widgets.
//!
//! The toolkit's `dark` palette is shared by every RustGames project, so
//! Mytherra keeps its own accent here rather than editing the toolkit. Chrome
//! and screens route their colours through this module, so the blue can be
//! tuned in one place (GDD 10 — a cohesive, presentable dashboard).

use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_text_right, draw_ui_text_ex, progress_bar};

// --- Palette (derived from the review mockup) --------------------------------

/// The deep blue-black canvas the whole client clears to.
pub const BG: Color = Color::new(0.043, 0.055, 0.078, 1.0);
/// A full panel's surface — lifted a touch from the canvas, with a blue tint.
pub const PANEL: Color = Color::new(0.078, 0.094, 0.126, 0.96);
/// A sub-card sitting inside a section (the dashboard grid cells).
pub const CARD: Color = Color::new(0.098, 0.118, 0.157, 1.0);
/// The strip behind a card / panel title.
pub const HEADER_BAR: Color = Color::new(0.118, 0.141, 0.190, 1.0);
/// The header / footer chrome band.
pub const CHROME: Color = Color::new(0.055, 0.070, 0.100, 0.98);

/// Section borders, and a softer variant for inner dividers.
pub const BORDER: Color = Color::new(0.30, 0.42, 0.62, 0.5);
pub const BORDER_SOFT: Color = Color::new(0.30, 0.42, 0.62, 0.28);

/// The signature blue — for active states, accents, and highlights.
pub const BLUE: Color = Color::new(0.36, 0.62, 1.0, 1.0);
/// A deep blue fill for the active nav tab.
pub const BLUE_DEEP: Color = Color::new(0.15, 0.28, 0.50, 1.0);
/// A bright top-edge highlight (the lit rim on the header and active tab).
pub const BLUE_EDGE: Color = Color::new(0.50, 0.72, 1.0, 0.9);
/// Warm gold — favour and other "divine" accents.
pub const GOLD: Color = Color::new(0.86, 0.70, 0.34, 1.0);

// --- Surface styles ----------------------------------------------------------

/// A full titled panel (the shell every screen draws its section into).
pub fn panel_style() -> SurfaceStyle {
    SurfaceStyle::new(PANEL)
        .with_border(1.0, BORDER)
        .with_header(42.0, HEADER_BAR)
        .with_header_divider(1.0, BORDER_SOFT)
}

/// A compact titled sub-card (the dashboard grid cells).
pub fn card_style() -> SurfaceStyle {
    SurfaceStyle::new(CARD)
        .with_border(1.0, BORDER_SOFT)
        .with_top_highlight(1.0, BORDER_SOFT)
        .with_header(32.0, HEADER_BAR)
        .with_header_divider(1.0, BORDER_SOFT)
}

/// Draw a compact titled sub-card and return its inner content rect (below the
/// header, inset on each side) for the caller to fill.
pub fn sub_card(rect: Rect, title: &str) -> Rect {
    draw_surface_with_title(
        rect,
        Some(title),
        &card_style(),
        TextStyle::new(16.0, dark::TEXT),
    );
    Rect::new(rect.x + 14.0, rect.y + 32.0, rect.w - 28.0, rect.h - 32.0)
}

/// The blue-forward nav-tab styling.
pub fn tab_style() -> TabStyle {
    TabStyle {
        active_fill: BLUE_DEEP,
        hover_fill: Color::new(0.11, 0.14, 0.20, 1.0),
        inactive_fill: Color::new(0.066, 0.082, 0.114, 1.0),
        border: Some((1.0, BORDER_SOFT)),
        active_accent: Some((2.5, BLUE_EDGE)),
        text_size: 17.0,
        active_text: dark::TEXT_BRIGHT,
        inactive_text: dark::TEXT_DIM,
        text_pad: 12.0,
    }
}

// --- Header chip -------------------------------------------------------------

/// A header resource chip: a coloured dot, a dim label, and a bright value,
/// drawn right-anchored. Returns the new right edge (its left, less a gap) so
/// chips can be laid out right-to-left like the mockup.
pub fn chip_right(right: f32, y: f32, w: f32, h: f32, dot: Color, label: &str, value: &str) -> f32 {
    let rect = Rect::new(right - w, y, w, h);
    draw_surface(rect, &rect_style());
    let dot_x = rect.x + 15.0;
    draw_circle(dot_x, rect.y + rect.h * 0.5, 5.0, dot);
    let text_x = dot_x + 15.0;
    draw_ui_text_ex(
        label,
        text_x,
        rect.y + 18.0,
        TextStyle::new(11.0, dark::TEXT_DIM).params(),
    );
    draw_ui_text_ex(
        value,
        text_x,
        rect.y + 35.0,
        TextStyle::new(17.0, dark::TEXT_BRIGHT).params(),
    );
    rect.x - 10.0
}

fn rect_style() -> SurfaceStyle {
    SurfaceStyle::new(CARD).with_border(1.0, BORDER_SOFT)
}

// --- Stat bar ----------------------------------------------------------------

/// A labelled stat row: a dim label at left, a bright value at right, and a thin
/// filled track beneath — the mockup's "World Status" look. Returns the next y.
pub fn stat_bar(
    area: Rect,
    y: f32,
    label: &str,
    value: f32,
    max: f32,
    color: Color,
    trend: &str,
) -> f32 {
    draw_ui_text_ex(
        label,
        area.x,
        y,
        TextStyle::new(14.0, dark::TEXT_DIM).params(),
    );
    draw_text_right(
        &format!("{value:.0}{trend}"),
        area.right(),
        y,
        TextStyle::new(14.0, dark::TEXT),
    );
    progress_bar(area.x, y + 6.0, area.w, 6.0, value, max, color);
    y + 26.0
}
