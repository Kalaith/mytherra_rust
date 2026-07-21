//! Myths: promote resonant tales into living myths that echo across the world
//! (GDD 5.6). Candidates on the left, living myths on the right.

use crate::data::fill;
use crate::ui::divine_tools::draw_panel;
use crate::ui::widgets::button;
use crate::ui::{UiAction, UiContext};
use crate::world::{Myth, MythCandidate};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt};

const MYTH_COLOR: Color = Color::new(0.7, 0.55, 0.9, 1.0);

pub fn draw(ctx: &UiContext<'_>, rect: Rect, actions: &mut Vec<UiAction>) {
    let left = Rect::new(rect.x, rect.y, (rect.w - 16.0) / 2.0, rect.h);
    let right = Rect::new(
        left.right() + 16.0,
        rect.y,
        rect.right() - left.right() - 16.0,
        rect.h,
    );
    draw_candidates(ctx, left, actions);
    draw_myths(ctx, right);
}

fn draw_candidates(ctx: &UiContext<'_>, rect: Rect, actions: &mut Vec<UiAction>) {
    let strings = &ctx.data.strings.divine;
    draw_panel(rect, &strings.myths_candidates);
    let content = rect.inset(16.0);

    if ctx.world.myth_candidates.is_empty() {
        draw_ui_text_ex(
            &strings.myths_no_candidates,
            content.x,
            content.y + 34.0,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
        return;
    }

    let mut y = content.y + 30.0;
    for candidate in ctx.world.myth_candidates.iter().take(5) {
        draw_candidate(
            ctx,
            candidate,
            Rect::new(content.x, y, content.w, 76.0),
            actions,
        );
        y += 84.0;
    }
}

fn draw_candidate(
    ctx: &UiContext<'_>,
    candidate: &MythCandidate,
    rect: Rect,
    actions: &mut Vec<UiAction>,
) {
    let strings = &ctx.data.strings.divine;
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.09, 0.1, 0.13, 1.0))
            .with_left_accent(4.0, MYTH_COLOR)
            .with_border(1.0, Color::new(0.4, 0.46, 0.58, 0.35)),
    );
    draw_ui_text_ex(
        &candidate.title,
        rect.x + 14.0,
        rect.y + 24.0,
        TextStyle::new(17.0, dark::TEXT_BRIGHT).params(),
    );
    draw_ui_text_ex(
        &fill(
            &strings.myth_meta,
            &[
                ("theme", candidate.theme_name.clone()),
                ("region", candidate.region_name.clone()),
            ],
        ),
        rect.x + 14.0,
        rect.y + 44.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );
    meter(
        Rect::new(rect.x + 14.0, rect.y + 54.0, rect.w - 160.0, 14.0),
        candidate.resonance,
        100.0,
        MYTH_COLOR,
        Some(&fill(
            &strings.myth_resonance,
            &[("resonance", format!("{:.0}", candidate.resonance))],
        )),
    );
    let cost = ctx.data.balance.myth.promote_cost;
    if button(
        Rect::new(rect.right() - 130.0, rect.y + 40.0, 116.0, 30.0),
        &fill(&strings.promote, &[("cost", cost.to_string())]),
        ctx.player.can_afford(cost),
        ButtonTone::Positive,
        ctx.mouse,
    ) {
        actions.push(UiAction::PromoteMyth(candidate.id.clone()));
    }
}

fn draw_myths(ctx: &UiContext<'_>, rect: Rect) {
    let strings = &ctx.data.strings.divine;
    let title = fill(
        &strings.myths_active,
        &[
            ("count", ctx.world.myths.len().to_string()),
            ("cap", ctx.data.balance.myth.cap.to_string()),
        ],
    );
    draw_panel(rect, &title);
    let content = rect.inset(16.0);

    if ctx.world.myths.is_empty() {
        draw_ui_text_ex(
            &strings.myths_no_myths,
            content.x,
            content.y + 34.0,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
        return;
    }

    let mut y = content.y + 30.0;
    for myth in ctx.world.myths.iter().take(6) {
        draw_myth(ctx, myth, Rect::new(content.x, y, content.w, 62.0));
        y += 70.0;
    }
}

fn draw_myth(ctx: &UiContext<'_>, myth: &Myth, rect: Rect) {
    let strings = &ctx.data.strings.divine;
    let threshold = ctx.data.balance.myth.echo_threshold;
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.09, 0.1, 0.13, 1.0))
            .with_left_accent(4.0, MYTH_COLOR)
            .with_border(1.0, Color::new(0.4, 0.46, 0.58, 0.35)),
    );
    draw_ui_text_ex(
        &myth.title,
        rect.x + 14.0,
        rect.y + 24.0,
        TextStyle::new(16.0, dark::TEXT).params(),
    );
    draw_ui_text_ex(
        &fill(
            &strings.myth_meta,
            &[
                ("theme", myth.theme_name.clone()),
                ("region", myth.region_name.clone()),
            ],
        ),
        rect.x + 14.0,
        rect.y + 44.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );

    let status = if myth.resonance < threshold {
        strings.myth_faint.clone()
    } else {
        fill(
            &strings.myth_echo_in,
            &[("years", myth.echo_cooldown.max(0).to_string())],
        )
    };
    let color = if myth.resonance < threshold {
        dark::TEXT_DIM
    } else {
        MYTH_COLOR
    };
    draw_ui_text_ex(
        &status,
        rect.right() - 160.0,
        rect.y + 32.0,
        TextStyle::new(14.0, color).params(),
    );

    // A myth still vivid enough to echo lifts the living heroes of its land
    // (GDD 5.6 <-> 5.4); note how many take heart, so the reach beyond the map
    // is visible on the board and not just in the chronicle.
    if myth.resonance >= threshold {
        let inspired = ctx
            .world
            .heroes
            .iter()
            .filter(|h| h.is_alive && h.region_id == myth.region_id)
            .count();
        if inspired > 0 {
            draw_ui_text_ex(
                &fill(&strings.myth_inspires, &[("count", inspired.to_string())]),
                rect.right() - 160.0,
                rect.y + 50.0,
                TextStyle::new(12.0, dark::TEXT_DIM).params(),
            );
        }
    }
}
