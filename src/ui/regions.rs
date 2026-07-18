//! Regions screen: a region roster on the left, detail + divine actions on the
//! right (GDD 10 "World Map / Regions").

use crate::data::{fill, RegionActionDef};
use crate::ui::widgets::{bad_stat_color, button, good_stat_color};
use crate::ui::{content_rect, UiAction, UiContext};
use crate::world::Region;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt};

pub fn draw(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let area = content_rect();
    let list = Rect::new(area.x, area.y, 360.0, area.h);
    let detail = Rect::new(
        list.right() + 16.0,
        area.y,
        area.right() - list.right() - 16.0,
        area.h,
    );

    draw_region_list(ctx, list, actions);
    draw_region_detail(ctx, detail, actions);
}

fn draw_region_list(ctx: &UiContext<'_>, rect: Rect, actions: &mut Vec<UiAction>) {
    draw_titled(rect, &ctx.data.strings.panels.regions);
    let content = rect.inset(16.0);
    let selected = selected_index(ctx);
    let mut y = content.y + 34.0;

    for (index, region) in ctx.world.regions.iter().enumerate() {
        let card = Rect::new(content.x, y, content.w, 68.0);
        let hovered = card.contains_point(ctx.mouse);
        let is_selected = index == selected;
        let fill_color = if is_selected {
            Color::new(0.15, 0.19, 0.26, 1.0)
        } else if hovered {
            Color::new(0.12, 0.14, 0.18, 1.0)
        } else {
            Color::new(0.09, 0.1, 0.13, 1.0)
        };
        let style = SurfaceStyle::new(fill_color)
            .with_left_accent(4.0, status_color(region))
            .with_border(1.0, Color::new(0.4, 0.46, 0.58, 0.35));
        draw_surface(card, &style);

        draw_ui_text_ex(
            &region.name,
            card.x + 16.0,
            card.y + 26.0,
            TextStyle::new(18.0, dark::TEXT_BRIGHT).params(),
        );
        draw_ui_text_ex(
            &fill(
                &ctx.data.strings.ui.region_subtitle,
                &[
                    ("status", region.status.label().to_owned()),
                    ("culture", region.culture.label().to_owned()),
                ],
            ),
            card.x + 16.0,
            card.y + 48.0,
            TextStyle::new(14.0, dark::TEXT_DIM).params(),
        );
        meter(
            Rect::new(card.right() - 108.0, card.y + 22.0, 92.0, 16.0),
            region.prosperity,
            100.0,
            good_stat_color(region.prosperity),
            None,
        );

        if hovered && is_mouse_button_released(MouseButton::Left) {
            actions.push(UiAction::SelectRegion(index));
        }
        y += 78.0;
    }
}

fn draw_region_detail(ctx: &UiContext<'_>, rect: Rect, actions: &mut Vec<UiAction>) {
    let strings = &ctx.data.strings;
    let selected = selected_index(ctx);
    let Some(region) = ctx.world.region(selected) else {
        draw_titled(rect, &strings.ui.no_region);
        return;
    };
    draw_titled(rect, &region.name);
    let content = rect.inset(18.0);
    let mut y = content.y + 38.0;

    // Descriptor badges.
    let mut bx = content.x;
    bx = badge(bx, y, 120.0, region.climate.label());
    bx = badge(bx, y, 120.0, region.culture.label());
    badge(bx, y, 130.0, region.status.label());
    y += 40.0;

    // Two columns of stat meters.
    let col_w = (content.w - 24.0) / 2.0;
    let left_x = content.x;
    let right_x = content.x + col_w + 24.0;
    let stats = &strings.stats;
    let mut ly = y;
    ly = stat(
        left_x,
        ly,
        col_w,
        &stats.prosperity,
        region.prosperity,
        good_stat_color(region.prosperity),
    );
    ly = stat(
        left_x,
        ly,
        col_w,
        &stats.chaos,
        region.chaos,
        bad_stat_color(region.chaos),
    );
    ly = stat(
        left_x,
        ly,
        col_w,
        &stats.danger,
        region.danger,
        bad_stat_color(region.danger),
    );
    let mut ry = y;
    ry = stat(
        right_x,
        ry,
        col_w,
        &stats.magic,
        region.magic_affinity,
        good_stat_color(region.magic_affinity),
    );
    ry = stat(
        right_x,
        ry,
        col_w,
        &stats.culture,
        region.cultural_influence,
        good_stat_color(region.cultural_influence),
    );
    ry = stat(
        right_x,
        ry,
        col_w,
        &stats.resonance,
        region.divine_resonance,
        good_stat_color(region.divine_resonance),
    );
    y = ly.max(ry) + 8.0;

    let region_balance = &ctx.data.balance.region;
    draw_ui_text_ex(
        &fill(
            &strings.ui.region_meta,
            &[
                ("pop", (region.population as i64).to_string()),
                (
                    "effect",
                    format!("{:.2}", region.effect_multiplier(region_balance)),
                ),
                (
                    "cost",
                    format!("{:.2}", region.cost_multiplier(region_balance)),
                ),
            ],
        ),
        content.x,
        y,
        TextStyle::new(14.0, dark::TEXT_DIM).params(),
    );
    y += 22.0;

    // Divine action buttons.
    draw_ui_text_ex(
        &strings.panels.divine_actions,
        content.x,
        y,
        TextStyle::new(18.0, dark::TEXT_BRIGHT).params(),
    );
    y += 14.0;
    for def in ctx.data.ordered_region_actions() {
        draw_action_card(
            ctx,
            region,
            def,
            Rect::new(content.x, y, content.w, 64.0),
            actions,
        );
        y += 72.0;
    }
}

fn draw_action_card(
    ctx: &UiContext<'_>,
    region: &Region,
    def: &RegionActionDef,
    rect: Rect,
    actions: &mut Vec<UiAction>,
) {
    let cost = region.action_cost(def, &ctx.data.balance.region);
    let affordable = ctx.player.can_afford(cost);
    let hovered = affordable && rect.contains_point(ctx.mouse);
    let fill_color = if hovered {
        Color::new(0.14, 0.17, 0.22, 1.0)
    } else {
        Color::new(0.1, 0.115, 0.145, 1.0)
    };
    let tone = action_tone(&def.id);
    draw_surface(
        rect,
        &SurfaceStyle::new(fill_color)
            .with_left_accent(4.0, ButtonStyle::from_tone(tone).normal)
            .with_border(1.0, Color::new(0.4, 0.46, 0.58, 0.35)),
    );
    draw_ui_text_ex(
        &def.name,
        rect.x + 16.0,
        rect.y + 24.0,
        TextStyle::new(
            17.0,
            if affordable {
                dark::TEXT
            } else {
                dark::TEXT_DIM
            },
        )
        .params(),
    );
    draw_ui_text_ex(
        &def.description,
        rect.x + 16.0,
        rect.y + 46.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );

    let btn = Rect::new(rect.right() - 118.0, rect.y + 14.0, 104.0, 36.0);
    let label = fill(
        &ctx.data.strings.ui.action_cost,
        &[("cost", cost.to_string())],
    );
    if button(btn, &label, affordable, tone, ctx.mouse) {
        actions.push(UiAction::RegionAction(def.id.clone()));
    }
}

fn action_tone(id: &str) -> ButtonTone {
    match id {
        "bless" => ButtonTone::Positive,
        "corrupt" => ButtonTone::Danger,
        _ => ButtonTone::Primary,
    }
}

fn selected_index(ctx: &UiContext<'_>) -> usize {
    ctx.selected_region
        .min(ctx.world.regions.len().saturating_sub(1))
}

fn stat(x: f32, y: f32, w: f32, label: &str, value: f32, color: Color) -> f32 {
    meter(
        Rect::new(x, y, w, 20.0),
        value,
        100.0,
        color,
        Some(&format!("{label} {value:.0}")),
    );
    y + 28.0
}

fn badge(x: f32, y: f32, w: f32, label: &str) -> f32 {
    let rect = Rect::new(x, y, w, 26.0);
    draw_badge(rect, label, Color::new(0.14, 0.17, 0.22, 1.0), dark::TEXT);
    rect.right() + 10.0
}

fn status_color(region: &Region) -> Color {
    use crate::world::RegionStatus::*;
    match region.status {
        Thriving => dark::POSITIVE,
        Prospering => Color::new(0.4, 0.7, 0.5, 1.0),
        Peaceful => dark::ACCENT,
        Unrest => dark::WARNING,
        Struggling => Color::new(0.85, 0.5, 0.3, 1.0),
        WarTorn => dark::NEGATIVE,
    }
}

fn draw_titled(rect: Rect, title: &str) {
    let style = SurfaceStyle::new(Color::new(0.07, 0.075, 0.095, 0.96))
        .with_border(1.0, Color::new(0.38, 0.45, 0.58, 0.5))
        .with_header(42.0, Color::new(0.1, 0.115, 0.145, 1.0))
        .with_header_divider(1.0, Color::new(0.38, 0.45, 0.58, 0.4));
    draw_surface_with_title(rect, Some(title), &style, TextStyle::new(20.0, dark::TEXT));
}
