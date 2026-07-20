//! Pantheon: the six deities in their ally/rival web (a light triangle opposed
//! to a shadow triangle), with Appease and Challenge actions gated on
//! relationship cooldowns (GDD 5.6).

use crate::data::fill;
use crate::data::strings::DivineText;
use crate::ui::divine_tools::draw_panel;
use crate::ui::widgets::button;
use crate::ui::{UiAction, UiContext};
use crate::world::PantheonDeity;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt};

pub fn draw(ctx: &UiContext<'_>, rect: Rect, actions: &mut Vec<UiAction>) {
    draw_panel(rect, &ctx.data.strings.divine.pantheon_panel);
    let content = rect.inset(16.0);

    // Two columns by three rows: the light triangle over the shadow triangle.
    let gap = 14.0;
    let rows = 3.0;
    let card_w = (content.w - gap) / 2.0;
    let card_h = (content.h - 24.0 - gap * (rows - 1.0)) / rows;
    for (index, deity) in ctx.world.pantheon.iter().enumerate().take(6) {
        let col = (index % 2) as f32;
        let row = (index / 2) as f32;
        let x = content.x + col * (card_w + gap);
        let y = content.y + 20.0 + row * (card_h + gap);
        draw_deity(ctx, deity, Rect::new(x, y, card_w, card_h), actions);
    }
}

fn draw_deity(ctx: &UiContext<'_>, deity: &PantheonDeity, rect: Rect, actions: &mut Vec<UiAction>) {
    let strings = &ctx.data.strings.divine;
    let balance = &ctx.data.balance.pantheon;
    let tier = deity.tier(balance);
    let accent = tier_color(tier);

    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.09, 0.1, 0.13, 1.0))
            .with_left_accent(4.0, accent)
            .with_border(1.0, Color::new(0.4, 0.46, 0.58, 0.35)),
    );
    draw_ui_text_ex(
        &deity.name,
        rect.x + 16.0,
        rect.y + 28.0,
        TextStyle::new(20.0, dark::TEXT_BRIGHT).params(),
    );
    draw_ui_text_ex(
        &fill(
            &strings.deity_meta,
            &[
                ("domain", deity.domain.clone()),
                ("ally", name_of(ctx, &deity.ally_id)),
                ("rival", name_of(ctx, &deity.rival_id)),
            ],
        ),
        rect.x + 16.0,
        rect.y + 50.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );

    meter(
        Rect::new(rect.x + 16.0, rect.y + 62.0, rect.w - 32.0, 18.0),
        deity.pressure,
        100.0,
        accent,
        Some(&fill(
            &strings.deity_pressure,
            &[
                ("mood", mood(tier, strings).to_owned()),
                ("pressure", format!("{:.0}", deity.pressure)),
            ],
        )),
    );

    // What this deity does to the world when roused, as a boon/bane badge in the
    // card's top-right — the player's cue for whom to appease (a rising bane) or
    // leave to stir (a boon), placed where it never crowds the layout (GDD 5.6).
    let boon = (deity.effect_amount > 0.0) == deity.effect_stat.rising_is_good();
    let verb = if deity.effect_amount >= 0.0 {
        &strings.verb_raises
    } else {
        &strings.verb_lowers
    };
    let effect = fill(
        &strings.deity_effect,
        &[
            ("verb", verb.clone()),
            ("stat", deity.effect_stat.label().to_owned()),
        ],
    );
    let badge_w = 148.0;
    draw_badge(
        Rect::new(rect.right() - badge_w - 14.0, rect.y + 14.0, badge_w, 22.0),
        &effect,
        Color::new(0.14, 0.16, 0.2, 1.0),
        if boon { dark::POSITIVE } else { dark::NEGATIVE },
    );

    // Appease / Challenge, or a resting note while on cooldown.
    let btn_y = rect.bottom() - 42.0;
    let half = (rect.w - 32.0 - 10.0) / 2.0;
    if deity.cooldown > 0 {
        draw_ui_text_ex(
            &fill(
                &strings.pantheon_cooldown,
                &[("years", deity.cooldown.to_string())],
            ),
            rect.x + 16.0,
            btn_y + 22.0,
            TextStyle::new(14.0, dark::TEXT_DIM).params(),
        );
        return;
    }
    if button(
        Rect::new(rect.x + 16.0, btn_y, half, 32.0),
        &fill(
            &strings.appease,
            &[("cost", balance.appease_cost.to_string())],
        ),
        ctx.player.can_afford(balance.appease_cost),
        ButtonTone::Positive,
        ctx.mouse,
    ) {
        actions.push(UiAction::AppeaseDeity(deity.id.clone()));
    }
    if button(
        Rect::new(rect.x + 26.0 + half, btn_y, half, 32.0),
        &fill(
            &strings.challenge,
            &[("cost", balance.challenge_cost.to_string())],
        ),
        ctx.player.can_afford(balance.challenge_cost),
        ButtonTone::Danger,
        ctx.mouse,
    ) {
        actions.push(UiAction::ChallengeDeity(deity.id.clone()));
    }
}

fn name_of(ctx: &UiContext<'_>, id: &str) -> String {
    ctx.world
        .pantheon
        .iter()
        .find(|d| d.id == id)
        .map(|d| d.name.clone())
        .unwrap_or_else(|| id.to_owned())
}

fn mood(tier: usize, strings: &DivineText) -> &str {
    match tier {
        0 => &strings.mood_dormant,
        1 => &strings.mood_stirring,
        2 => &strings.mood_roused,
        3 => &strings.mood_wrathful,
        _ => &strings.mood_ascendant,
    }
}

fn tier_color(tier: usize) -> Color {
    match tier {
        0 => dark::TEXT_DIM,
        1 => dark::ACCENT,
        2 => dark::WARNING,
        _ => dark::NEGATIVE,
    }
}
