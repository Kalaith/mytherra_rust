//! Artifacts: forge new relics and tend existing ones (GDD 5.6).

use crate::data::fill;
use crate::ui::divine_tools::draw_panel;
use crate::ui::widgets::{bad_stat_color, button};
use crate::ui::{UiAction, UiContext};
use crate::world::Artifact;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt};

pub fn draw(ctx: &UiContext<'_>, rect: Rect, actions: &mut Vec<UiAction>) {
    let strings = &ctx.data.strings.divine;
    let balance = &ctx.data.balance.artifact;
    let title = fill(
        &strings.artifacts_panel,
        &[
            ("count", ctx.world.artifacts.len().to_string()),
            ("max", balance.max_active.to_string()),
        ],
    );
    draw_panel(rect, &title);
    let content = rect.inset(16.0);

    // Forge row: focus selector + forge button.
    let row_y = content.y + 26.0;
    if button(
        Rect::new(content.x, row_y, 220.0, 32.0),
        &fill(
            &strings.create_focus,
            &[("focus", ctx.create_focus.label().to_owned())],
        ),
        true,
        ButtonTone::Secondary,
        ctx.mouse,
    ) {
        actions.push(UiAction::CycleArtifactFocus);
    }
    let full = ctx.world.artifacts.len() >= balance.max_active;
    let can_create = !full && ctx.player.can_afford(balance.create_cost);
    if button(
        Rect::new(content.x + 232.0, row_y, 170.0, 32.0),
        &fill(
            &strings.create,
            &[("cost", balance.create_cost.to_string())],
        ),
        can_create,
        ButtonTone::Positive,
        ctx.mouse,
    ) {
        actions.push(UiAction::CreateArtifact);
    }

    if ctx.world.artifacts.is_empty() {
        draw_ui_text_ex(
            &strings.artifacts_empty,
            content.x,
            content.y + 84.0,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
        return;
    }

    // Two columns of artifact cards.
    let top = content.y + 74.0;
    let gap = 16.0;
    let card_w = (content.w - gap) / 2.0;
    let card_h = 104.0;
    for (index, artifact) in ctx.world.artifacts.iter().enumerate().take(8) {
        let col = (index % 2) as f32;
        let row = (index / 2) as f32;
        let x = content.x + col * (card_w + gap);
        let y = top + row * (card_h + 10.0);
        draw_card(ctx, artifact, Rect::new(x, y, card_w, card_h), actions);
    }
}

fn draw_card(ctx: &UiContext<'_>, artifact: &Artifact, rect: Rect, actions: &mut Vec<UiAction>) {
    let strings = &ctx.data.strings.divine;
    let balance = &ctx.data.balance.artifact;
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.09, 0.1, 0.13, 1.0))
            .with_left_accent(4.0, Color::new(0.6, 0.3, 0.9, 1.0))
            .with_border(1.0, Color::new(0.4, 0.46, 0.58, 0.35)),
    );
    draw_ui_text_ex(
        &artifact.name,
        rect.x + 14.0,
        rect.y + 24.0,
        TextStyle::new(18.0, dark::TEXT_BRIGHT).params(),
    );
    let region = ctx
        .world
        .region_name(&artifact.region_id)
        .unwrap_or(&artifact.region_id);
    draw_ui_text_ex(
        &fill(
            &strings.artifact_meta,
            &[
                ("focus", artifact.focus.label().to_owned()),
                ("power", artifact.power.to_string()),
                ("region", region.to_owned()),
            ],
        ),
        rect.x + 14.0,
        rect.y + 44.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );
    // Colour and label the instability by proximity to the backlash threshold
    // (not an absolute 0-100 scale), so the warning is correct for any tuning and
    // the player is told to stabilize before a relic shatters (GDD 5.6).
    let proximity = artifact.instability / balance.backlash_threshold.max(1.0);
    let critical = proximity >= 0.7;
    let template = if critical {
        &strings.instability_critical
    } else {
        &strings.instability
    };
    meter(
        Rect::new(rect.x + 14.0, rect.y + 54.0, rect.w - 28.0, 12.0),
        artifact.instability,
        balance.backlash_threshold,
        if critical {
            dark::NEGATIVE
        } else {
            bad_stat_color(proximity * 100.0)
        },
        Some(&fill(
            template,
            &[("instability", format!("{:.0}", artifact.instability))],
        )),
    );

    // Empower / Stabilize / Move buttons.
    let btn_y = rect.bottom() - 34.0;
    let btn_w = (rect.w - 28.0 - 16.0) / 3.0;
    let empower_cost = artifact.empower_cost(balance);
    if button(
        Rect::new(rect.x + 14.0, btn_y, btn_w, 28.0),
        &fill(&strings.empower, &[("cost", empower_cost.to_string())]),
        ctx.player.can_afford(empower_cost),
        ButtonTone::Primary,
        ctx.mouse,
    ) {
        actions.push(UiAction::EmpowerArtifact(artifact.id.clone()));
    }
    if button(
        Rect::new(rect.x + 22.0 + btn_w, btn_y, btn_w, 28.0),
        &fill(
            &strings.stabilize,
            &[("cost", balance.stabilize_cost.to_string())],
        ),
        ctx.player.can_afford(balance.stabilize_cost),
        ButtonTone::Positive,
        ctx.mouse,
    ) {
        actions.push(UiAction::StabilizeArtifact(artifact.id.clone()));
    }
    if button(
        Rect::new(rect.x + 30.0 + btn_w * 2.0, btn_y, btn_w, 28.0),
        &fill(
            &strings.transfer,
            &[("cost", balance.transfer_cost.to_string())],
        ),
        ctx.player.can_afford(balance.transfer_cost),
        ButtonTone::Secondary,
        ctx.mouse,
    ) {
        actions.push(UiAction::TransferArtifact(artifact.id.clone()));
    }
}
