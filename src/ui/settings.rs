//! Settings (GDD 10): pacing controls (auto-tick cadence + pause) and read-only
//! world info. A pure view — the controls emit `SetTickSpeed` / `TogglePause`
//! intents; `Game` owns the actual pacing state.

use crate::data::fill;
use crate::ui::widgets::{button, draw_titled};
use crate::ui::{content_rect, UiAction, UiContext};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt};

pub fn draw(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let strings = &ctx.data.strings.settings;
    let area = content_rect();
    draw_titled(area, &strings.panel);
    let content = area.inset(24.0);
    let mut y = content.y + 34.0;

    // --- World tick speed ---------------------------------------------------
    draw_ui_text_ex(
        &strings.tick_speed_title,
        content.x,
        y,
        TextStyle::new(18.0, dark::TEXT_BRIGHT).params(),
    );
    y += 20.0;
    draw_ui_text_ex(
        &strings.tick_speed_hint,
        content.x,
        y,
        TextStyle::new(14.0, dark::TEXT_DIM).params(),
    );
    y += 22.0;

    let presets = &ctx.data.balance.settings.tick_speed_presets;
    let chip_w = 92.0;
    let gap = 10.0;
    for (index, seconds) in presets.iter().enumerate() {
        let rect = Rect::new(content.x + index as f32 * (chip_w + gap), y, chip_w, 34.0);
        let tone = if index == ctx.tick_speed_index {
            ButtonTone::Primary
        } else {
            ButtonTone::Secondary
        };
        let label = fill(&strings.speed_chip, &[("seconds", format!("{seconds:.0}"))]);
        if button(rect, &label, true, tone, ctx.mouse) {
            actions.push(UiAction::SetTickSpeed(index));
        }
    }
    y += 62.0;

    // --- Pacing (pause / resume) --------------------------------------------
    draw_ui_text_ex(
        &strings.pacing_title,
        content.x,
        y,
        TextStyle::new(18.0, dark::TEXT_BRIGHT).params(),
    );
    y += 24.0;
    let (btn_label, tone, status) = if ctx.paused {
        (
            &strings.resume,
            ButtonTone::Positive,
            &strings.status_paused,
        )
    } else {
        (
            &strings.pause,
            ButtonTone::Secondary,
            &strings.status_running,
        )
    };
    if button(
        Rect::new(content.x, y, 176.0, 38.0),
        btn_label,
        true,
        tone,
        ctx.mouse,
    ) {
        actions.push(UiAction::TogglePause);
    }
    draw_ui_text_ex(
        status,
        content.x + 196.0,
        y + 24.0,
        TextStyle::new(15.0, dark::TEXT_DIM).params(),
    );
    y += 64.0;

    // --- Read-only world info -----------------------------------------------
    draw_ui_text_ex(
        &strings.world_title,
        content.x,
        y,
        TextStyle::new(18.0, dark::TEXT_BRIGHT).params(),
    );
    y += 24.0;
    let config = &ctx.data.config;
    for line in [
        fill(
            &strings.info_display,
            &[("name", config.display_name.clone())],
        ),
        fill(
            &strings.info_version,
            &[("version", config.version.clone())],
        ),
        fill(
            &strings.info_seed,
            &[("seed", config.world_seed.to_string())],
        ),
        fill(
            &strings.info_year,
            &[
                ("year", ctx.world.year.to_string()),
                ("regions", ctx.world.regions.len().to_string()),
            ],
        ),
    ] {
        draw_ui_text_ex(
            &line,
            content.x,
            y,
            TextStyle::new(15.0, dark::TEXT).params(),
        );
        y += 24.0;
    }
}
