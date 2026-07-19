//! Civilization: the six agendas competing within the selected region, and the
//! Advance action to press one (GDD 5.6).

use crate::data::fill;
use crate::ui::divine_tools::draw_panel;
use crate::ui::widgets::button;
use crate::ui::{UiAction, UiContext};
use crate::world::{agenda_score, dominant_agenda, RegionAgendas};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt};

pub fn draw(ctx: &UiContext<'_>, rect: Rect, actions: &mut Vec<UiAction>) {
    let strings = &ctx.data.strings.divine;
    draw_panel(rect, &strings.civ_panel);
    let content = rect.inset(16.0);

    let region_index = ctx
        .selected_region
        .min(ctx.world.regions.len().saturating_sub(1));
    let Some(region) = ctx.world.region(region_index) else {
        return;
    };
    let empty = RegionAgendas::new(region.id.clone(), ctx.data.agendas.len());
    let entry = ctx
        .world
        .civilization
        .iter()
        .find(|e| e.region_id == region.id)
        .unwrap_or(&empty);

    // Region selector (cycles the shared selection) + diplomacy cooldown.
    if button(
        Rect::new(content.x, content.y + 26.0, 240.0, 30.0),
        &fill(&strings.civ_region, &[("region", region.name.clone())]),
        ctx.world.regions.len() > 1,
        ButtonTone::Secondary,
        ctx.mouse,
    ) {
        let next = (region_index + 1) % ctx.world.regions.len();
        actions.push(UiAction::SelectRegion(next));
    }
    let cooldown_text = if entry.cooldown > 0 {
        fill(
            &strings.civ_cooldown,
            &[("years", entry.cooldown.to_string())],
        )
    } else {
        strings.civ_ready.clone()
    };
    draw_ui_text_ex(
        &cooldown_text,
        content.x + 256.0,
        content.y + 46.0,
        TextStyle::new(14.0, dark::TEXT_DIM).params(),
    );

    let threshold = ctx.data.balance.civilization.apply_threshold;
    let cost = ctx.data.balance.civilization.advance_cost;
    let can_advance = entry.cooldown == 0 && ctx.player.can_afford(cost);

    // Only the region's dominant agenda is the one it actually pursues.
    let dominant = dominant_agenda(&ctx.data.agendas, region, entry, threshold);
    let mut y = content.y + 68.0;
    for (index, agenda) in ctx.data.agendas.iter().enumerate() {
        let score = agenda_score(agenda, region, entry.boost(index));
        let active = dominant == Some(index);
        draw_agenda(
            ctx,
            &agenda.name,
            score,
            active,
            cost,
            can_advance,
            index,
            Rect::new(content.x, y, content.w, 50.0),
            actions,
        );
        y += 58.0;
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_agenda(
    ctx: &UiContext<'_>,
    name: &str,
    score: f32,
    active: bool,
    cost: i64,
    can_advance: bool,
    index: usize,
    rect: Rect,
    actions: &mut Vec<UiAction>,
) {
    let strings = &ctx.data.strings.divine;
    let accent = if active {
        dark::POSITIVE
    } else {
        dark::TEXT_DIM
    };
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.09, 0.1, 0.13, 1.0))
            .with_left_accent(4.0, accent)
            .with_border(1.0, Color::new(0.4, 0.46, 0.58, 0.35)),
    );

    let meter_w = rect.w - 320.0;
    meter(
        Rect::new(rect.x + 14.0, rect.y + 17.0, meter_w, 16.0),
        score.clamp(0.0, 100.0),
        100.0,
        if active { dark::POSITIVE } else { dark::ACCENT },
        Some(&fill(
            &strings.agenda_score,
            &[("name", name.to_owned()), ("score", format!("{score:.0}"))],
        )),
    );

    let status = if active {
        &strings.agenda_active
    } else {
        &strings.agenda_dormant
    };
    draw_badge(
        Rect::new(rect.right() - 250.0, rect.y + 12.0, 92.0, 24.0),
        status,
        Color::new(0.14, 0.16, 0.2, 1.0),
        accent,
    );

    if button(
        Rect::new(rect.right() - 144.0, rect.y + 10.0, 130.0, 30.0),
        &fill(&strings.advance, &[("cost", cost.to_string())]),
        can_advance,
        ButtonTone::Primary,
        ctx.mouse,
    ) {
        actions.push(UiAction::AdvanceAgenda(index));
    }
}
