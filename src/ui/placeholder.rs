//! A simple "coming soon" panel for screens whose systems aren't built yet.

use crate::ui::{content_rect, UiContext};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::draw_ui_text_ex;

pub fn draw(_ctx: &UiContext<'_>, title: &str, body: &str) {
    let rect = content_rect();
    let style = SurfaceStyle::new(Color::new(0.07, 0.075, 0.095, 0.96))
        .with_border(1.0, Color::new(0.38, 0.45, 0.58, 0.5))
        .with_header(44.0, Color::new(0.1, 0.115, 0.145, 1.0))
        .with_header_divider(1.0, Color::new(0.38, 0.45, 0.58, 0.4));
    draw_surface_with_title(rect, Some(title), &style, TextStyle::new(20.0, dark::TEXT));

    draw_ui_text_ex(
        body,
        rect.x + 24.0,
        rect.y + 88.0,
        TextStyle::new(17.0, dark::TEXT_DIM).params(),
    );
}
