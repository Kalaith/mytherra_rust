//! Eras screen: the present age and its trigger pressures on the left, the
//! chronicle of past eras on the right (GDD 5.7, 10).

use crate::data::fill;
use crate::ui::widgets::bad_stat_color;
use crate::ui::{content_rect, UiContext};
use crate::world::compute_scores;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt};

pub fn draw(ctx: &UiContext<'_>) {
    let area = content_rect();
    let left = Rect::new(area.x, area.y, 604.0, area.h);
    let right = Rect::new(
        left.right() + 16.0,
        area.y,
        area.right() - left.right() - 16.0,
        area.h,
    );
    draw_present(ctx, left);
    draw_history(ctx, right);
}

fn draw_present(ctx: &UiContext<'_>, rect: Rect) {
    let strings = &ctx.data.strings.eras;
    draw_titled(rect, &strings.current_title);
    let content = rect.inset(18.0);
    let era = &ctx.world.era;
    let balance = &ctx.data.balance.era;

    draw_ui_text_ex(
        &fill(
            &strings.era_line,
            &[
                ("number", era.number.to_string()),
                ("name", era.name.clone()),
            ],
        ),
        content.x,
        content.y + 36.0,
        TextStyle::new(22.0, dark::TEXT_BRIGHT).params(),
    );
    draw_ui_text_ex(
        &fill(&strings.since, &[("year", era.start_year.to_string())]),
        content.x,
        content.y + 58.0,
        TextStyle::new(14.0, dark::TEXT_DIM).params(),
    );

    let breaking = era.pressure >= balance.breaking_threshold;
    meter(
        Rect::new(content.x, content.y + 72.0, content.w, 24.0),
        era.pressure,
        balance.breaking_threshold,
        bad_stat_color(era.pressure / balance.breaking_threshold * 100.0),
        Some(&fill(
            &strings.pressure,
            &[("pressure", format!("{:.0}", era.pressure))],
        )),
    );
    draw_ui_text_ex(
        if breaking {
            &strings.breaking
        } else {
            &strings.holding
        },
        content.x,
        content.y + 118.0,
        TextStyle::new(
            14.0,
            if breaking {
                dark::NEGATIVE
            } else {
                dark::TEXT_DIM
            },
        )
        .params(),
    );

    // Trigger breakdown.
    draw_ui_text_ex(
        &strings.triggers_title,
        content.x,
        content.y + 150.0,
        TextStyle::new(18.0, dark::TEXT_BRIGHT).params(),
    );
    let pending_stake: i64 = ctx
        .player
        .bets
        .iter()
        .filter(|b| b.resolved.is_none())
        .map(|b| b.stake)
        .sum();
    let scores = compute_scores(
        &ctx.world.regions,
        &ctx.world.heroes,
        &ctx.world.magic_paths,
        ctx.player.favor,
        ctx.data.config.max_favor,
        pending_stake,
        ctx.world.conquest_momentum,
        balance,
    );
    let dominant = scores.dominant().0;
    let mut y = content.y + 168.0;
    for (trigger, score) in scores.all() {
        let is_dominant = trigger == dominant;
        meter(
            Rect::new(content.x, y, content.w, 20.0),
            score,
            balance.breaking_threshold,
            if is_dominant {
                dark::WARNING
            } else {
                dark::ACCENT
            },
            Some(&format!("{}  {:.0}", trigger.label(), score)),
        );
        y += 28.0;
    }
}

fn draw_history(ctx: &UiContext<'_>, rect: Rect) {
    let strings = &ctx.data.strings.eras;
    draw_titled(rect, &strings.history_title);
    let content = rect.inset(18.0);

    if ctx.world.era_history.is_empty() {
        draw_ui_text_ex(
            &strings.no_history,
            content.x,
            content.y + 34.0,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
        return;
    }

    let mut y = content.y + 32.0;
    for record in ctx.world.era_history.iter().rev() {
        draw_ui_text_ex(
            &fill(
                &strings.record_line,
                &[
                    ("number", record.number.to_string()),
                    ("name", record.name.clone()),
                ],
            ),
            content.x,
            y,
            TextStyle::new(16.0, dark::TEXT_BRIGHT).params(),
        );
        draw_ui_text_ex(
            &fill(
                &strings.record_span,
                &[
                    ("start", record.start_year.to_string()),
                    ("end", record.end_year.to_string()),
                    ("trigger", record.trigger.label().to_owned()),
                    ("pressure", format!("{:.0}", record.pressure)),
                ],
            ),
            content.x,
            y + 20.0,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
        );
        y += 52.0;
        if y > content.bottom() {
            break;
        }
    }
}

fn draw_titled(rect: Rect, title: &str) {
    let style = SurfaceStyle::new(Color::new(0.07, 0.075, 0.095, 0.96))
        .with_border(1.0, Color::new(0.38, 0.45, 0.58, 0.5))
        .with_header(42.0, Color::new(0.1, 0.115, 0.145, 1.0))
        .with_header_divider(1.0, Color::new(0.38, 0.45, 0.58, 0.4));
    draw_surface_with_title(rect, Some(title), &style, TextStyle::new(20.0, dark::TEXT));
}
