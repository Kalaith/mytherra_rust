//! Omens: a read-only forecast across three horizons (GDD 5.6) — the far
//! horizon of the coming age, and each region's near pressure and its
//! generational drift. Omens never mutate world state.

use crate::data::fill;
use crate::data::strings::DivineText;
use crate::ui::divine_tools::draw_panel;
use crate::ui::widgets::bad_stat_color;
use crate::ui::UiContext;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt};

pub fn draw(ctx: &UiContext<'_>, rect: Rect) {
    let strings = &ctx.data.strings.divine;
    draw_panel(rect, &strings.omens_panel);
    let content = rect.inset(18.0);

    // Far horizon (GDD 5.6): which age the world is building toward. The near and
    // generational horizons follow, region by region.
    let era = &ctx.world.era;
    draw_ui_text_ex(
        &fill(
            &strings.omens_intro,
            &[
                ("trigger", era.dominant_trigger.label().to_owned()),
                ("pressure", format!("{:.0}", era.pressure)),
            ],
        ),
        content.x,
        content.y + 30.0,
        TextStyle::new(15.0, dark::TEXT_DIM).params(),
    );

    let omens = &ctx.data.balance.omens;
    let mut y = content.y + 52.0;
    for region in &ctx.world.regions {
        let pressure = region.pressure();
        draw_ui_text_ex(
            &region.name,
            content.x,
            y + 20.0,
            TextStyle::new(17.0, dark::TEXT_BRIGHT).params(),
        );
        meter(
            Rect::new(content.x, y + 30.0, content.w, 20.0),
            pressure,
            100.0,
            bad_stat_color(pressure),
            Some(&fill(
                &strings.omen_line,
                &[
                    ("pressure", format!("{pressure:.0}")),
                    ("tier", tier(pressure, strings).to_owned()),
                ],
            )),
        );

        // Generational horizon (GDD 5.6): extrapolate the current pressure drift
        // forward — a read-only projection, never a change to the world.
        let drift = pressure - region.prev_pressure();
        let projected = (pressure + drift * omens.horizon_ticks).clamp(0.0, 100.0);
        let outlook = if drift > omens.trend_deadzone {
            &strings.omen_deepening
        } else if drift < -omens.trend_deadzone {
            &strings.omen_easing
        } else {
            &strings.omen_holding
        };
        draw_ui_text_ex(
            &fill(
                &strings.omen_horizon,
                &[
                    ("outlook", outlook.clone()),
                    ("tier", tier(projected, strings).to_owned()),
                ],
            ),
            content.x,
            y + 66.0,
            TextStyle::new(13.0, bad_stat_color(projected)).params(),
        );

        // The divine works currently shaping this region — omens surface cause,
        // never change it.
        let relics = ctx
            .world
            .artifacts
            .iter()
            .filter(|a| a.region_id == region.id)
            .count();
        let storms = ctx
            .world
            .weather
            .iter()
            .filter(|w| w.region_id == region.id)
            .count();
        let myths = ctx
            .world
            .myths
            .iter()
            .filter(|m| m.region_id == region.id)
            .count();
        let forces = if relics + storms + myths == 0 {
            strings.omen_no_forces.clone()
        } else {
            fill(
                &strings.omen_forces,
                &[
                    ("relics", relics.to_string()),
                    ("storms", storms.to_string()),
                    ("myths", myths.to_string()),
                ],
            )
        };
        draw_ui_text_ex(
            &forces,
            content.x,
            y + 86.0,
            TextStyle::new(14.0, dark::TEXT_DIM).params(),
        );
        y += 102.0;
    }
}

/// Qualitative omen tier from a region's pressure reading.
fn tier(pressure: f32, strings: &DivineText) -> &str {
    if pressure < 30.0 {
        &strings.omen_calm
    } else if pressure < 55.0 {
        &strings.omen_stirring
    } else if pressure < 75.0 {
        &strings.omen_turbulent
    } else {
        &strings.omen_dire
    }
}
