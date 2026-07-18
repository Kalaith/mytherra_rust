//! Divine Observatory: speculation events with live house+crowd odds on the
//! left, the player's wagers on the right (GDD 5.5, 10).

use crate::data::fill;
use crate::ui::widgets::button;
use crate::ui::{content_rect, UiAction, UiContext};
use crate::world::{quote_event, Bet, SpeculationEvent};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt};

pub fn draw(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let area = content_rect();
    let left = Rect::new(area.x, area.y, 762.0, area.h);
    let right = Rect::new(
        left.right() + 16.0,
        area.y,
        area.right() - left.right() - 16.0,
        area.h,
    );

    draw_events_panel(ctx, left, actions);
    draw_bets_panel(ctx, right);
}

fn selected_stake(ctx: &UiContext<'_>) -> i64 {
    let presets = &ctx.data.balance.betting.stake_presets;
    presets[ctx.bet_stake_index.min(presets.len() - 1)]
}

fn draw_events_panel(ctx: &UiContext<'_>, rect: Rect, actions: &mut Vec<UiAction>) {
    let strings = &ctx.data.strings.betting;
    draw_titled(rect, &strings.panel_events);
    let content = rect.inset(16.0);

    // Confidence + stake selectors.
    let confidence =
        &ctx.data.confidence_levels[ctx.bet_confidence.min(ctx.data.confidence_levels.len() - 1)];
    let stake = selected_stake(ctx);
    if button(
        Rect::new(content.x, content.y + 24.0, 200.0, 32.0),
        &fill(
            &strings.confidence_btn,
            &[("confidence", confidence.name.clone())],
        ),
        true,
        ButtonTone::Secondary,
        ctx.mouse,
    ) {
        actions.push(UiAction::CycleConfidence);
    }
    if button(
        Rect::new(content.x + 212.0, content.y + 24.0, 150.0, 32.0),
        &fill(&strings.stake_btn, &[("stake", stake.to_string())]),
        true,
        ButtonTone::Secondary,
        ctx.mouse,
    ) {
        actions.push(UiAction::CycleStake);
    }

    let active: Vec<&SpeculationEvent> = ctx
        .world
        .speculations
        .iter()
        .filter(|e| e.is_active())
        .collect();
    if active.is_empty() {
        draw_ui_text_ex(
            &strings.no_events,
            content.x,
            content.y + 84.0,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
        return;
    }

    let mut y = content.y + 70.0;
    for event in active.iter().take(5) {
        draw_event_card(
            ctx,
            event,
            confidence,
            stake,
            Rect::new(content.x, y, content.w, 82.0),
            actions,
        );
        y += 90.0;
    }
}

fn draw_event_card(
    ctx: &UiContext<'_>,
    event: &SpeculationEvent,
    confidence: &crate::data::ConfidenceLevel,
    stake: i64,
    rect: Rect,
    actions: &mut Vec<UiAction>,
) {
    let strings = &ctx.data.strings.betting;
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.09, 0.1, 0.13, 1.0))
            .with_left_accent(4.0, dark::ACCENT)
            .with_border(1.0, Color::new(0.4, 0.46, 0.58, 0.35)),
    );
    draw_ui_text_ex(
        &event.description,
        rect.x + 14.0,
        rect.y + 24.0,
        TextStyle::new(17.0, dark::TEXT_BRIGHT).params(),
    );
    draw_ui_text_ex(
        &format!(
            "{}  ·  {}",
            event.bet_type_name,
            fill(
                &strings.deadline,
                &[("deadline", event.deadline_year.to_string())]
            )
        ),
        rect.x + 14.0,
        rect.y + 46.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );

    let likelihood = event.likelihood(
        &ctx.world.heroes,
        &ctx.world.regions,
        &ctx.world.settlements,
    );
    let quote = quote_event(
        event,
        likelihood,
        confidence,
        stake,
        &ctx.data.balance.betting,
    );
    draw_ui_text_ex(
        &fill(
            &strings.odds,
            &[
                ("odds", format!("{:.2}", quote.odds)),
                ("crowd", format!("{:.0}", quote.crowd_pct)),
            ],
        ),
        rect.x + 14.0,
        rect.y + 68.0,
        TextStyle::new(13.0, dark::WARNING).params(),
    );

    let btn = Rect::new(rect.right() - 224.0, rect.y + 24.0, 210.0, 36.0);
    let can = ctx.player.can_afford(stake);
    if button(
        btn,
        &fill(
            &strings.place,
            &[
                ("stake", stake.to_string()),
                ("payout", quote.payout.to_string()),
            ],
        ),
        can,
        ButtonTone::Positive,
        ctx.mouse,
    ) {
        actions.push(UiAction::PlaceBet(event.id.clone()));
    }
}

fn draw_bets_panel(ctx: &UiContext<'_>, rect: Rect) {
    let strings = &ctx.data.strings.betting;
    draw_titled(rect, &strings.panel_bets);
    let content = rect.inset(16.0);

    if ctx.player.bets.is_empty() {
        draw_ui_text_ex(
            &strings.no_bets,
            content.x,
            content.y + 30.0,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
        return;
    }

    let mut y = content.y + 26.0;
    for bet in ctx.player.bets.iter().rev().take(8) {
        draw_bet_card(ctx, bet, Rect::new(content.x, y, content.w, 56.0));
        y += 64.0;
    }
}

fn draw_bet_card(ctx: &UiContext<'_>, bet: &Bet, rect: Rect) {
    let strings = &ctx.data.strings.betting;
    let (status, color) = match bet.resolved {
        None => (&strings.pending, dark::ACCENT),
        Some(true) => (&strings.won, dark::POSITIVE),
        Some(false) => (&strings.lost, dark::NEGATIVE),
    };
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.09, 0.1, 0.13, 1.0))
            .with_left_accent(4.0, color)
            .with_border(1.0, Color::new(0.4, 0.46, 0.58, 0.35)),
    );
    draw_ui_text_ex(
        &fill(
            &strings.bet_line,
            &[
                ("bet_type", bet.bet_type_name.clone()),
                ("target", bet.target_name.clone()),
            ],
        ),
        rect.x + 14.0,
        rect.y + 22.0,
        TextStyle::new(15.0, dark::TEXT).params(),
    );
    draw_ui_text_ex(
        &fill(
            &strings.bet_meta,
            &[
                ("confidence", bet.confidence_name.clone()),
                ("stake", bet.stake.to_string()),
                ("payout", bet.potential_payout.to_string()),
                ("deadline", bet.deadline_year.to_string()),
            ],
        ),
        rect.x + 14.0,
        rect.y + 42.0,
        TextStyle::new(12.0, dark::TEXT_DIM).params(),
    );
    draw_badge(
        Rect::new(rect.right() - 84.0, rect.y + 14.0, 70.0, 22.0),
        status,
        Color::new(0.14, 0.16, 0.2, 1.0),
        color,
    );
}

fn draw_titled(rect: Rect, title: &str) {
    let style = SurfaceStyle::new(Color::new(0.07, 0.075, 0.095, 0.96))
        .with_border(1.0, Color::new(0.38, 0.45, 0.58, 0.5))
        .with_header(42.0, Color::new(0.1, 0.115, 0.145, 1.0))
        .with_header_divider(1.0, Color::new(0.38, 0.45, 0.58, 0.4));
    draw_surface_with_title(rect, Some(title), &style, TextStyle::new(20.0, dark::TEXT));
}
