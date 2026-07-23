//! The title / main menu screen (GDD 10): the first thing a returning deity
//! sees, offering a fresh world, a saved one, settings, and the door out. Stands
//! alone — no header, nav, or footer wrap it.

use crate::ui::widgets::button;
use crate::ui::{Screen, UiAction, UiContext, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, measure_ui_text};

pub fn draw(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let t = &ctx.data.strings.title;
    let cx = LOGICAL_WIDTH * 0.5;

    // A framed panel over the background, so the menu reads as its own place.
    let panel = Rect::new(cx - 260.0, 90.0, 520.0, LOGICAL_HEIGHT - 180.0);
    draw_surface(
        panel,
        &SurfaceStyle::new(Color::new(0.07, 0.08, 0.11, 0.96))
            .with_border(1.5, Color::new(0.3, 0.5, 0.8, 0.6)),
    );

    centered(&t.game_title, cx, panel.y + 96.0, 68.0, dark::TEXT_BRIGHT);
    centered(&t.tagline, cx, panel.y + 148.0, 16.0, dark::TEXT_DIM);

    // Menu buttons, centred and stacked.
    let btn_w = 300.0;
    let btn_h = 52.0;
    let gap = 16.0;
    let x = cx - btn_w * 0.5;
    let mut y = panel.y + 210.0;

    if button(
        Rect::new(x, y, btn_w, btn_h),
        &t.enter_world,
        true,
        ButtonTone::Positive,
        ctx.mouse,
    ) {
        actions.push(UiAction::EnterWorld);
    }
    // A quiet note that the world lives on a server, not this machine.
    centered(&t.online_note, cx, y + btn_h + 12.0, 12.0, dark::TEXT_DIM);
    y += btn_h + gap;

    if button(
        Rect::new(x, y, btn_w, btn_h),
        &t.settings,
        true,
        ButtonTone::Secondary,
        ctx.mouse,
    ) {
        actions.push(UiAction::SelectScreen(Screen::Settings));
    }
    y += btn_h + gap;

    if button(
        Rect::new(x, y, btn_w, btn_h),
        &t.exit,
        true,
        ButtonTone::Secondary,
        ctx.mouse,
    ) {
        actions.push(UiAction::ExitGame);
    }
}

/// Draw text horizontally centred on `cx`, at the given baseline `y`.
fn centered(text: &str, cx: f32, y: f32, size: f32, color: Color) {
    let width = measure_ui_text(text, None, size.round() as u16, 1.0).width;
    draw_ui_text_ex(
        text,
        cx - width * 0.5,
        y,
        TextStyle::new(size, color).params(),
    );
}
