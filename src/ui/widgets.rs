//! Project-local, mouse-aware UI helpers.
//!
//! The toolkit's own `button_rect_*` / `tab_bar` hit-test against the raw screen
//! mouse, which is wrong inside a letterboxed `VirtualUi` frame. These helpers
//! take the already-transformed logical mouse position, mirroring the pattern
//! the starter template established with its `virtual_button`. Drawing still
//! goes through toolkit surfaces/text so the look stays consistent.

use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::RectExt;

/// A clickable button that hit-tests against the logical `mouse` position.
pub fn button(rect: Rect, text: &str, enabled: bool, tone: ButtonTone, mouse: Vec2) -> bool {
    let style = ButtonStyle::from_tone(tone);
    let hovered = enabled && rect.contains_point(mouse);
    let pressed = hovered && is_mouse_button_down(MouseButton::Left);
    let activated = hovered && is_mouse_button_released(MouseButton::Left);

    let fill = if !enabled {
        style.disabled
    } else if pressed {
        style.pressed
    } else if hovered {
        style.hovered
    } else {
        style.normal
    };
    draw_surface(
        rect,
        &SurfaceStyle::new(fill).with_border(1.5, style.border),
    );
    draw_text_centered_in_box_ex(
        text,
        rect.x + 8.0,
        rect.y + if pressed { 2.0 } else { 0.0 },
        rect.w - 16.0,
        rect.h,
        TextStyle::new(
            17.0,
            if enabled {
                style.text_color
            } else {
                dark::TEXT_DIM
            },
        ),
    );
    activated
}

/// A one-of-N horizontal tab bar. Returns the clicked index this frame, if any.
pub fn nav_tabs(rect: Rect, labels: &[&str], active: usize, mouse: Vec2) -> Option<usize> {
    if labels.is_empty() {
        return None;
    }
    let count = labels.len() as f32;
    let tab_w = rect.w / count;
    let mut clicked = None;

    for (index, label) in labels.iter().enumerate() {
        let tab = Rect::new(rect.x + index as f32 * tab_w, rect.y, tab_w, rect.h);
        let is_active = index == active;
        let hovered = tab.contains_point(mouse);
        let fill = if is_active {
            Color::new(0.16, 0.22, 0.32, 1.0)
        } else if hovered {
            Color::new(0.12, 0.14, 0.18, 1.0)
        } else {
            Color::new(0.08, 0.09, 0.12, 1.0)
        };
        let mut style = SurfaceStyle::new(fill).with_border(1.0, Color::new(0.3, 0.36, 0.46, 0.5));
        if is_active {
            style = style.with_top_highlight(3.0, dark::ACCENT);
        }
        draw_surface(tab, &style);
        draw_text_centered_in_box_ex(
            label,
            tab.x + 4.0,
            tab.y,
            tab.w - 8.0,
            tab.h,
            TextStyle::new(
                17.0,
                if is_active {
                    dark::TEXT_BRIGHT
                } else {
                    dark::TEXT_DIM
                },
            ),
        );
        if hovered && is_mouse_button_released(MouseButton::Left) {
            clicked = Some(index);
        }
    }
    clicked
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
