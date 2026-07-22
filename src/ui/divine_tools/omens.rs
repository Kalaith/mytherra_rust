//! Omens: a read-only forecast across three horizons (GDD 5.6) — the far
//! horizon of the coming age, and each region's near pressure and its
//! generational drift. Omens never mutate world state.

use crate::data::fill;
use crate::data::strings::DivineText;
use crate::ui::divine_tools::draw_panel;
use crate::ui::widgets::{bad_stat_color, page_controls, paginate};
use crate::ui::{UiAction, UiContext};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt};

pub fn draw(ctx: &UiContext<'_>, rect: Rect, actions: &mut Vec<UiAction>) {
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

    // Page the region forecasts: region genesis grows the map past what one panel
    // can hold, so drawing every block would spill off the bottom (GDD 5.2 <-> 10).
    let omens = &ctx.data.balance.omens;
    let list_top = content.y + 52.0;
    let stride = 102.0;
    let pager_row = Rect::new(content.x, content.bottom() - 26.0, content.w, 24.0);
    let page_size = (((pager_row.y - 6.0 - list_top) / stride).floor() as usize).max(1);
    let (page, start, end, total_pages) =
        paginate(ctx.world.regions.len(), page_size, ctx.omens_page);

    let mut y = list_top;
    for region in ctx.world.regions.iter().take(end).skip(start) {
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
        // A scheduled consequence is the most concrete omen of all: fold the
        // soonest one bound for this region into the horizon, foretelling a
        // coming scar or harvest by name and timing (GDD 5.6).
        let coming = ctx
            .world
            .pending_consequences
            .iter()
            .filter(|c| c.region_id == region.id)
            .min_by_key(|c| c.delay)
            .map(|c| {
                let tmpl = if c.effect.is_boon() {
                    &strings.omen_coming_harvest
                } else {
                    &strings.omen_coming_scar
                };
                fill(
                    tmpl,
                    &[
                        ("source", c.source.clone()),
                        ("years", c.delay.max(0).to_string()),
                    ],
                )
            })
            .unwrap_or_default();
        draw_ui_text_ex(
            &fill(
                &strings.omen_horizon,
                &[
                    ("outlook", outlook.clone()),
                    ("tier", tier(projected, strings).to_owned()),
                    ("coming", coming),
                ],
            ),
            content.x,
            y + 66.0,
            TextStyle::new(13.0, bad_stat_color(projected)).params(),
        );

        // A present war, plague, or beast is the most concrete omen of all — it
        // takes the forces slot in a warning hue, ahead of the divine-work tally,
        // when a land is under threat (GDD 5.6 <-> 5.3/5.2).
        let mut threats: Vec<&str> = Vec::new();
        if ctx
            .world
            .wars
            .iter()
            .any(|w| w.aggressor_id == region.id || w.defender_id == region.id)
        {
            threats.push(&strings.omen_war);
        }
        if ctx.world.plagues.iter().any(|p| p.region_id == region.id) {
            threats.push(&strings.omen_plague);
        }
        if ctx.world.monsters.iter().any(|m| m.region_id == region.id) {
            threats.push(&strings.omen_beast);
        }
        if threats.is_empty() {
            // The divine works currently shaping this region — omens surface
            // cause, never change it.
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
        } else {
            draw_ui_text_ex(
                &threats.join("   ·   "),
                content.x,
                y + 86.0,
                TextStyle::new(14.0, bad_stat_color(90.0)).params(),
            );
        }
        y += stride;
    }

    if total_pages > 1 {
        let ui = &ctx.data.strings.ui;
        let label = fill(
            &ui.page_label,
            &[
                ("page", (page + 1).to_string()),
                ("pages", total_pages.to_string()),
            ],
        );
        if let Some(target) = page_controls(
            pager_row,
            page,
            total_pages,
            &ui.page_prev,
            &ui.page_next,
            &label,
            ctx.mouse,
        ) {
            actions.push(UiAction::SetOmensPage(target));
        }
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
