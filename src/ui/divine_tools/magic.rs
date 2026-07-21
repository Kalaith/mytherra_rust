//! Magic: five research paths maturing toward Known, each with a Research
//! action to accelerate it (GDD 5.6).

use crate::data::fill;
use crate::data::strings::DivineText;
use crate::data::HeroRole;
use crate::ui::divine_tools::draw_panel;
use crate::ui::widgets::button;
use crate::ui::{UiAction, UiContext};
use crate::world::{MagicPath, MagicState};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt};

pub fn draw(ctx: &UiContext<'_>, rect: Rect, actions: &mut Vec<UiAction>) {
    let strings = &ctx.data.strings.divine;
    draw_panel(rect, &strings.magic_panel);
    let content = rect.inset(16.0);

    // Start below the 40px panel header so the title stays clear.
    let mut y = content.y + 26.0;
    draw_ui_text_ex(
        &strings.magic_intro,
        content.x,
        y,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );
    y += 18.0;

    // The scholarly momentum now driving every path (GDD 5.6 <-> 5.4): surface
    // why research quickens, so the player can read the world's learned minds as
    // a lever — cultivate scholars and mages to master the arcane sooner.
    let learned = ctx
        .world
        .heroes
        .iter()
        .filter(|h| h.is_alive && matches!(h.role, HeroRole::Scholar | HeroRole::Mage))
        .count();
    let (momentum, momentum_color) = if learned > 0 {
        (
            fill(&strings.magic_scholars, &[("count", learned.to_string())]),
            Color::new(0.6, 0.55, 0.9, 1.0),
        )
    } else {
        (strings.magic_no_scholars.clone(), dark::TEXT_DIM)
    };
    draw_ui_text_ex(
        &momentum,
        content.x,
        y,
        TextStyle::new(14.0, momentum_color).params(),
    );
    y += 22.0;

    // Knowledge relics are a second lever on research beside scholars (GDD 5.6):
    // note them when any exist, so the player can read forging Knowledge relics as
    // a way to hasten the arcane — the Artifacts tool feeding the Magic tool.
    let relics = ctx
        .world
        .artifacts
        .iter()
        .filter(|a| a.focus == crate::data::ArtifactFocus::Knowledge)
        .count();
    if relics > 0 {
        draw_ui_text_ex(
            &fill(&strings.magic_relics, &[("count", relics.to_string())]),
            content.x,
            y,
            TextStyle::new(14.0, Color::new(0.6, 0.55, 0.9, 1.0)).params(),
        );
        y += 22.0;
    }

    for path in &ctx.world.magic_paths {
        draw_path(ctx, path, Rect::new(content.x, y, content.w, 74.0), actions);
        y += 80.0;
    }
}

fn draw_path(ctx: &UiContext<'_>, path: &MagicPath, rect: Rect, actions: &mut Vec<UiAction>) {
    let strings = &ctx.data.strings.divine;
    let balance = &ctx.data.balance.magic;
    let (state_label, state_color) = state_style(path.state, strings);

    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.09, 0.1, 0.13, 1.0))
            .with_left_accent(4.0, state_color)
            .with_border(1.0, Color::new(0.4, 0.46, 0.58, 0.35)),
    );
    draw_ui_text_ex(
        &path.name,
        rect.x + 14.0,
        rect.y + 24.0,
        TextStyle::new(18.0, dark::TEXT_BRIGHT).params(),
    );
    draw_ui_text_ex(
        &path.description,
        rect.x + 14.0,
        rect.y + 44.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );

    // Two meters filling the left area, right area reserved for badge + button.
    let right_w = 150.0;
    let left_w = rect.w - right_w - 40.0;
    let meter_w = (left_w - 10.0) / 2.0;
    meter(
        Rect::new(rect.x + 14.0, rect.y + 56.0, meter_w, 14.0),
        path.progress,
        balance.known_progress,
        dark::ACCENT,
        Some(&fill(
            &strings.magic_progress,
            &[("progress", format!("{:.0}", path.progress))],
        )),
    );
    meter(
        Rect::new(rect.x + 24.0 + meter_w, rect.y + 56.0, meter_w, 14.0),
        path.evidence,
        balance.stat_cap,
        Color::new(0.6, 0.55, 0.9, 1.0),
        Some(&fill(
            &strings.magic_evidence,
            &[("evidence", format!("{:.0}", path.evidence))],
        )),
    );

    // Right column: state badge + research button.
    let rx = rect.right() - right_w - 14.0;
    draw_badge(
        Rect::new(rx, rect.y + 14.0, right_w, 22.0),
        state_label,
        Color::new(0.14, 0.16, 0.2, 1.0),
        state_color,
    );
    if button(
        Rect::new(rx, rect.y + 42.0, right_w, 30.0),
        &fill(
            &strings.research,
            &[("cost", balance.research_cost.to_string())],
        ),
        ctx.player.can_afford(balance.research_cost),
        ButtonTone::Primary,
        ctx.mouse,
    ) {
        actions.push(UiAction::ResearchMagic(path.id.clone()));
    }
}

fn state_style(state: MagicState, strings: &DivineText) -> (&str, Color) {
    match state {
        MagicState::Dormant => (&strings.magic_dormant, dark::TEXT_DIM),
        MagicState::Emerging => (&strings.magic_emerging, dark::WARNING),
        MagicState::Known => (&strings.magic_known, dark::POSITIVE),
    }
}
