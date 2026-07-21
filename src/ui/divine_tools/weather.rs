//! Weather: shape a pattern over the selected region and watch active fronts
//! decay (GDD 5.6).

use crate::data::fill;
use crate::ui::divine_tools::draw_panel;
use crate::ui::widgets::button;
use crate::ui::{UiAction, UiContext};
use crate::world::{weather_cost, WeatherEvent};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt};

pub fn draw(ctx: &UiContext<'_>, rect: Rect, actions: &mut Vec<UiAction>) {
    let strings = &ctx.data.strings.divine;
    draw_panel(rect, &strings.weather_panel);
    let content = rect.inset(16.0);

    let pattern =
        &ctx.data.weather_patterns[ctx.weather_pattern.min(ctx.data.weather_patterns.len() - 1)];
    let intensity = &ctx.data.weather_intensities[ctx
        .weather_intensity
        .min(ctx.data.weather_intensities.len() - 1)];

    // Pattern + intensity selectors.
    let row_y = content.y + 24.0;
    if button(
        Rect::new(content.x, row_y, 220.0, 32.0),
        &fill(
            &strings.weather_pattern,
            &[("pattern", pattern.name.clone())],
        ),
        true,
        ButtonTone::Secondary,
        ctx.mouse,
    ) {
        actions.push(UiAction::CycleWeatherPattern);
    }
    if button(
        Rect::new(content.x + 232.0, row_y, 200.0, 32.0),
        &fill(
            &strings.weather_intensity,
            &[("intensity", intensity.name.clone())],
        ),
        true,
        ButtonTone::Secondary,
        ctx.mouse,
    ) {
        actions.push(UiAction::CycleWeatherIntensity);
    }

    // Shape button targets the currently selected region.
    let region_index = ctx
        .selected_region
        .min(ctx.world.regions.len().saturating_sub(1));
    if let Some(region) = ctx.world.region(region_index) {
        let cost = weather_cost(
            ctx.data.balance.weather.base_cost,
            intensity.cost_mult,
            region.cost_multiplier(&ctx.data.balance.region),
        );
        let full = ctx.world.weather.len() >= ctx.data.balance.weather.max_active;
        if button(
            Rect::new(content.x + 444.0, row_y, 300.0, 32.0),
            &fill(
                &strings.shape,
                &[("region", region.name.clone()), ("cost", cost.to_string())],
            ),
            !full && ctx.player.can_afford(cost),
            ButtonTone::Positive,
            ctx.mouse,
        ) {
            actions.push(UiAction::ShapeWeather);
        }
    }

    if ctx.world.weather.is_empty() {
        draw_ui_text_ex(
            &strings.weather_empty,
            content.x,
            content.y + 84.0,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
        return;
    }

    let mut y = content.y + 74.0;
    for front in ctx.world.weather.iter().take(6) {
        draw_front(ctx, front, Rect::new(content.x, y, content.w, 62.0));
        y += 70.0;
    }
}

fn draw_front(ctx: &UiContext<'_>, front: &WeatherEvent, rect: Rect) {
    let strings = &ctx.data.strings.divine;
    let region = ctx
        .world
        .region_name(&front.region_id)
        .unwrap_or(&front.region_id);
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.09, 0.1, 0.13, 1.0))
            .with_left_accent(4.0, dark::ACCENT)
            .with_border(1.0, Color::new(0.4, 0.46, 0.58, 0.35)),
    );
    draw_ui_text_ex(
        &fill(
            &strings.weather_meta,
            &[
                ("pattern", front.pattern_name.clone()),
                ("intensity", front.intensity_name.clone()),
                ("region", region.to_owned()),
            ],
        ),
        rect.x + 14.0,
        rect.y + 24.0,
        TextStyle::new(16.0, dark::TEXT).params(),
    );

    // Whether this front blesses or blights the land it sits on. A badge, so a
    // glance tells fair weather from foul (GDD 5.6).
    let boon = front.is_fair();
    draw_badge(
        Rect::new(rect.right() - 124.0, rect.y + 12.0, 110.0, 22.0),
        if boon {
            &strings.weather_boon
        } else {
            &strings.weather_bane
        },
        Color::new(0.14, 0.16, 0.2, 1.0),
        if boon { dark::POSITIVE } else { dark::NEGATIVE },
    );
    meter(
        Rect::new(rect.x + 14.0, rect.y + 36.0, rect.w - 28.0, 14.0),
        front.magnitude,
        3.5,
        dark::ACCENT,
        Some(&fill(
            &strings.weather_magnitude,
            &[("magnitude", format!("{:.1}", front.magnitude))],
        )),
    );
}
