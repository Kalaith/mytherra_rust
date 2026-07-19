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
use macroquad_toolkit::ui::draw_ui_text_ex;

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

/// Clamp `requested` to a valid page over `count` items and return
/// `(page, start, end, total_pages)`. `page_size` is floored at 1; an empty list
/// still yields one (empty) page so callers never divide by zero. Shared by every
/// paged list so they all clamp identically.
pub fn paginate(count: usize, page_size: usize, requested: usize) -> (usize, usize, usize, usize) {
    let page_size = page_size.max(1);
    let total_pages = count.div_ceil(page_size).max(1);
    let page = requested.min(total_pages - 1);
    let start = page * page_size;
    let end = (start + page_size).min(count);
    (page, start, end, total_pages)
}

/// Right-aligned prev/label/next page controls drawn on `row`. `page_label` is
/// the already-formatted readout (e.g. "Page 2 / 5"). Returns the target page a
/// nav button requested this frame, if any — always an exact, in-range value, so
/// a stale stored page can never strand the reader.
#[allow(clippy::too_many_arguments)]
pub fn page_controls(
    row: Rect,
    page: usize,
    total_pages: usize,
    prev_label: &str,
    next_label: &str,
    page_label: &str,
    mouse: Vec2,
) -> Option<usize> {
    let bw = 84.0;
    let h = row.h;
    let label_w = 116.0;
    let next = Rect::new(row.right() - bw, row.y, bw, h);
    let label_x = next.x - 10.0 - label_w;
    let prev = Rect::new(label_x - 10.0 - bw, row.y, bw, h);

    let mut target = None;
    if button(prev, prev_label, page > 0, ButtonTone::Secondary, mouse) {
        target = Some(page - 1);
    }
    draw_ui_text_ex(
        page_label,
        label_x,
        row.y + h - 8.0,
        TextStyle::new(14.0, dark::TEXT_DIM).params(),
    );
    if button(
        next,
        next_label,
        page + 1 < total_pages,
        ButtonTone::Secondary,
        mouse,
    ) {
        target = Some(page + 1);
    }
    target
}

#[cfg(test)]
mod tests {
    use super::paginate;

    #[test]
    fn first_page_starts_at_the_newest() {
        assert_eq!(paginate(30, 14, 0), (0, 0, 14, 3));
    }

    #[test]
    fn a_middle_and_last_page_slice_correctly() {
        assert_eq!(paginate(30, 14, 1), (1, 14, 28, 3));
        assert_eq!(paginate(30, 14, 2), (2, 28, 30, 3));
    }

    #[test]
    fn an_overshot_request_clamps_to_the_last_page() {
        assert_eq!(paginate(30, 14, 99), (2, 28, 30, 3));
    }

    #[test]
    fn an_empty_list_is_one_empty_page() {
        assert_eq!(paginate(0, 14, 3), (0, 0, 0, 1));
    }

    #[test]
    fn a_zero_page_size_never_divides_by_zero() {
        assert_eq!(paginate(5, 0, 0), (0, 0, 1, 5));
    }
}
