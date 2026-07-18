//! Dashboard: world-at-a-glance, the player's standing, and the recent chronicle.

use crate::data::fill;
use crate::ui::widgets::{bad_stat_color, button, good_stat_color};
use crate::ui::{content_rect, UiAction, UiContext};
use crate::world::EventKind;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt};

pub fn draw(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let area = content_rect();
    let left = Rect::new(area.x, area.y, 604.0, area.h);
    let right = Rect::new(
        left.right() + 16.0,
        area.y,
        area.right() - left.right() - 16.0,
        area.h,
    );

    draw_world_panel(ctx, left, actions);
    draw_chronicle_panel(ctx, right);
}

fn draw_world_panel(ctx: &UiContext<'_>, rect: Rect, actions: &mut Vec<UiAction>) {
    let strings = &ctx.data.strings;
    draw_titled(rect, &strings.panels.world);
    let content = rect.inset(18.0);
    let mut y = content.y + 40.0;

    let summary = ctx.world.summary();
    let stats = &strings.stats;
    y = stat_row(
        content,
        y,
        &stats.prosperity,
        summary.avg_prosperity,
        good_stat_color(summary.avg_prosperity),
    );
    y = stat_row(
        content,
        y,
        &stats.chaos,
        summary.avg_chaos,
        bad_stat_color(summary.avg_chaos),
    );
    y = stat_row(
        content,
        y,
        &stats.danger,
        summary.avg_danger,
        bad_stat_color(summary.avg_danger),
    );
    y = stat_row(
        content,
        y,
        &stats.magic,
        summary.avg_magic,
        good_stat_color(summary.avg_magic),
    );
    y += 6.0;

    draw_ui_text_ex(
        &fill(
            &strings.ui.world_summary,
            &[
                ("regions", summary.region_count.to_string()),
                ("crisis", summary.regions_in_crisis.to_string()),
                ("souls", format_population(summary.total_population)),
            ],
        ),
        content.x,
        y + 6.0,
        TextStyle::new(15.0, dark::TEXT_DIM).params(),
    );
    y += 34.0;

    // Player standing.
    draw_ui_text_ex(
        &strings.panels.standing,
        content.x,
        y,
        TextStyle::new(18.0, dark::TEXT_BRIGHT).params(),
    );
    y += 14.0;
    meter(
        Rect::new(content.x, y, content.w, 22.0),
        ctx.player.favor as f32,
        ctx.data.config.max_favor as f32,
        dark::POSITIVE,
        Some(&fill(
            &strings.ui.favor_meter,
            &[
                ("favor", ctx.player.favor.to_string()),
                ("max", ctx.data.config.max_favor.to_string()),
            ],
        )),
    );
    y += 30.0;
    let next_cost = ctx.player.next_level_cost(&ctx.data.balance.player);
    meter(
        Rect::new(content.x, y, content.w, 22.0),
        ctx.player.experience as f32,
        next_cost as f32,
        dark::ACCENT,
        Some(&fill(
            &strings.ui.level_meter,
            &[
                ("level", ctx.player.level.to_string()),
                ("xp", ctx.player.experience.to_string()),
                ("next", next_cost.to_string()),
            ],
        )),
    );
    y += 32.0;
    draw_ui_text_ex(
        &fill(
            &strings.ui.standing_summary,
            &[
                ("nudges", ctx.player.nudges.to_string()),
                ("spent", ctx.player.favor_spent.to_string()),
            ],
        ),
        content.x,
        y,
        TextStyle::new(15.0, dark::TEXT_DIM).params(),
    );
    y += 36.0;

    // Era panel (GDD 10): the present age and its pressure.
    let era = &ctx.world.era;
    let era_balance = &ctx.data.balance.era;
    draw_ui_text_ex(
        &strings.eras.current_title,
        content.x,
        y,
        TextStyle::new(18.0, dark::TEXT_BRIGHT).params(),
    );
    y += 22.0;
    draw_ui_text_ex(
        &fill(
            &strings.eras.era_line,
            &[
                ("number", era.number.to_string()),
                ("name", era.name.clone()),
            ],
        ),
        content.x,
        y,
        TextStyle::new(15.0, dark::TEXT).params(),
    );
    y += 14.0;
    let breaking = era.pressure >= era_balance.breaking_threshold;
    meter(
        Rect::new(content.x, y, content.w, 22.0),
        era.pressure,
        era_balance.breaking_threshold,
        bad_stat_color(era.pressure / era_balance.breaking_threshold * 100.0),
        Some(&fill(
            &strings.eras.pressure,
            &[("pressure", format!("{:.0}", era.pressure))],
        )),
    );
    y += 28.0;
    draw_ui_text_ex(
        if breaking {
            &strings.eras.breaking
        } else {
            &strings.eras.holding
        },
        content.x,
        y,
        TextStyle::new(
            13.0,
            if breaking {
                dark::NEGATIVE
            } else {
                dark::TEXT_DIM
            },
        )
        .params(),
    );

    // Save / new-world controls anchored at the bottom of the panel.
    let btn_y = rect.bottom() - 52.0;
    let btn_w = (content.w - 24.0) / 3.0;
    if button(
        Rect::new(content.x, btn_y, btn_w, 36.0),
        &strings.ui.save,
        true,
        ButtonTone::Positive,
        ctx.mouse,
    ) {
        actions.push(UiAction::Save);
    }
    if button(
        Rect::new(content.x + btn_w + 12.0, btn_y, btn_w, 36.0),
        &strings.ui.load,
        ctx.save_exists,
        ButtonTone::Primary,
        ctx.mouse,
    ) {
        actions.push(UiAction::Load);
    }
    if button(
        Rect::new(content.x + (btn_w + 12.0) * 2.0, btn_y, btn_w, 36.0),
        &strings.ui.new_world,
        true,
        ButtonTone::Secondary,
        ctx.mouse,
    ) {
        actions.push(UiAction::NewWorld);
    }
}

fn draw_chronicle_panel(ctx: &UiContext<'_>, rect: Rect) {
    draw_titled(rect, &ctx.data.strings.panels.chronicle);
    let content = rect.inset(18.0);
    let mut y = content.y + 40.0;

    if ctx.world.chronicle.is_empty() {
        draw_ui_text_ex(
            &ctx.data.strings.ui.empty_chronicle,
            content.x,
            y,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
        return;
    }

    for event in ctx.world.chronicle.recent(16) {
        let color = kind_color(event.kind);
        draw_badge(
            Rect::new(content.x, y - 15.0, 74.0, 20.0),
            &format!("Y{}", event.year),
            Color::new(0.14, 0.16, 0.2, 1.0),
            color,
        );
        draw_ui_text_ex(
            &event.message,
            content.x + 84.0,
            y,
            TextStyle::new(15.0, dark::TEXT).params(),
        );
        y += 26.0;
        if y > content.bottom() {
            break;
        }
    }
}

fn stat_row(content: Rect, y: f32, label: &str, value: f32, color: Color) -> f32 {
    meter(
        Rect::new(content.x, y, content.w, 22.0),
        value,
        100.0,
        color,
        Some(&format!("{label}  {value:.0}")),
    );
    y + 30.0
}

fn draw_titled(rect: Rect, title: &str) {
    let style = SurfaceStyle::new(Color::new(0.07, 0.075, 0.095, 0.96))
        .with_border(1.0, Color::new(0.38, 0.45, 0.58, 0.5))
        .with_header(42.0, Color::new(0.1, 0.115, 0.145, 1.0))
        .with_header_divider(1.0, Color::new(0.38, 0.45, 0.58, 0.4));
    draw_surface_with_title(rect, Some(title), &style, TextStyle::new(20.0, dark::TEXT));
}

fn kind_color(kind: EventKind) -> Color {
    match kind {
        EventKind::Tick => dark::TEXT_DIM,
        EventKind::Divine => dark::ACCENT,
        EventKind::Region => dark::WARNING,
        EventKind::Hero => Color::new(0.7, 0.55, 0.9, 1.0),
        EventKind::System => dark::POSITIVE,
    }
}

fn format_population(pop: f32) -> String {
    if pop >= 1_000_000.0 {
        format!("{:.1}M", pop / 1_000_000.0)
    } else if pop >= 1_000.0 {
        format!("{:.1}k", pop / 1_000.0)
    } else {
        format!("{pop:.0}")
    }
}
