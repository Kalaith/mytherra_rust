//! Dashboard: the world at a glance, the deity's standing, and its heroes.
//!
//! Laid out as the review mockup's card grid — a 2×2 of sub-cards (Your
//! Standing / World Status / The Present Age / Divine Actions) beside a Recent
//! Events rail, with a Heroes-of-Note strip along the foot. The Divine Actions
//! buttons are shortcuts: they navigate to where each verb already lives
//! (Regions / Divine Tools / Observatory), so the dashboard adds no new game
//! logic — it only presents (GDD 10).

use crate::data::{fill, HeroRole};
use crate::ui::theme;
use crate::ui::widgets::{
    bad_stat_color, button, draw_titled, good_stat_color, trend_marker,
};
use crate::ui::{content_rect, Screen, UiAction, UiContext};
use crate::world::{EventKind, Hero};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt};

/// How many living heroes the foot strip spotlights.
const HERO_SPOTLIGHT: usize = 5;

pub fn draw(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let area = content_rect();

    // Foot strip: the heroes of note, spanning the full width.
    let strip = Rect::new(area.x, area.bottom() - 116.0, area.w, 116.0);
    // Everything above it: the card grid on the left, the events rail on the right.
    let main_h = strip.y - 12.0 - area.y;
    let left = Rect::new(area.x, area.y, 604.0, main_h);
    let right = Rect::new(
        left.right() + 16.0,
        area.y,
        area.right() - left.right() - 16.0,
        main_h,
    );

    draw_card_grid(ctx, left, actions);
    draw_events_rail(ctx, right, actions);
    draw_heroes_strip(ctx, strip, actions);
}

/// The 2×2 sub-card grid filling the left rail.
fn draw_card_grid(ctx: &UiContext<'_>, area: Rect, actions: &mut Vec<UiAction>) {
    let gap = 16.0;
    let cw = (area.w - gap) / 2.0;
    let ch = (area.h - gap) / 2.0;
    let col2 = area.x + cw + gap;
    let row2 = area.y + ch + gap;

    draw_standing_card(ctx, Rect::new(area.x, area.y, cw, ch));
    draw_world_status_card(ctx, Rect::new(col2, area.y, cw, ch));
    draw_present_age_card(ctx, Rect::new(area.x, row2, cw, ch));
    draw_actions_card(ctx, Rect::new(col2, row2, cw, ch), actions);
}

fn draw_standing_card(ctx: &UiContext<'_>, rect: Rect) {
    let strings = &ctx.data.strings;
    let c = theme::sub_card(rect, &strings.panels.standing);
    let mut y = c.y + 12.0;

    // Favour ceiling and per-tick income come pre-computed from the projection
    // (§7.7): income is the standing's base recovery plus the tithe the faithful
    // lands pour back, summed over the *full* world — true even when the view
    // hides the tithing regions.
    meter(
        Rect::new(c.x, y, c.w, 22.0),
        ctx.player.favor as f32,
        ctx.max_favor as f32,
        theme::GOLD,
        Some(&fill(
            &strings.ui.favor_meter,
            &[
                ("favor", ctx.player.favor.to_string()),
                ("max", ctx.max_favor.to_string()),
                ("income", ctx.favor_income.to_string()),
            ],
        )),
    );
    y += 30.0;

    let next_cost = ctx.player.next_level_cost(&ctx.data.balance.player);
    meter(
        Rect::new(c.x, y, c.w, 22.0),
        ctx.player.experience as f32,
        next_cost as f32,
        theme::BLUE,
        Some(&fill(
            &strings.ui.level_meter,
            &[
                ("level", ctx.player.level.to_string()),
                ("xp", ctx.player.experience.to_string()),
                ("next", next_cost.to_string()),
            ],
        )),
    );
    y += 34.0;

    let (done, total) = ctx.player.achievements.progress();
    draw_ui_text_ex(
        &fill(
            &strings.ui.standing_summary,
            &[
                ("nudges", ctx.player.nudges.to_string()),
                ("spent", ctx.player.favor_spent.to_string()),
                ("goals", done.to_string()),
                ("total", total.to_string()),
            ],
        ),
        c.x,
        y,
        TextStyle::new(14.0, dark::TEXT_DIM).params(),
    );
    y += 20.0;

    // The world's accumulated works — a world with a history (GDD 10).
    let legends = living_legends(ctx);
    draw_ui_text_ex(
        &fill(
            &strings.ui.world_works,
            &[
                ("towns", ctx.world.settlements.len().to_string()),
                ("wonders", ctx.world.landmarks.len().to_string()),
                ("legends", legends.to_string()),
            ],
        ),
        c.x,
        y,
        TextStyle::new(14.0, dark::TEXT_DIM).params(),
    );
}

fn draw_world_status_card(ctx: &UiContext<'_>, rect: Rect) {
    let strings = &ctx.data.strings;
    let c = theme::sub_card(rect, &strings.panels.world_status);
    let summary = ctx.world.summary;
    let mut y = c.y + 14.0;

    // The age's tenor: a qualitative read of the world's health (GDD 10).
    let tenor = &ctx.data.balance.tenor;
    let labels = &strings.ui.tenor_labels;
    let tier = summary
        .tenor(&tenor.thresholds, tenor.crisis_penalty)
        .min(labels.len().saturating_sub(1));
    if let Some(label) = labels.get(tier) {
        draw_ui_text_ex(
            &fill(&strings.ui.tenor_line, &[("tenor", label.clone())]),
            c.x,
            y,
            TextStyle::new(13.0, tenor_color(tier, labels.len())).params(),
        );
        y += 22.0;
    }

    let stats = &strings.stats;
    y = theme::stat_bar(
        c,
        y,
        &stats.prosperity,
        summary.avg_prosperity,
        100.0,
        good_stat_color(summary.avg_prosperity),
        trend_marker(summary.trend_prosperity),
    );
    y = theme::stat_bar(
        c,
        y,
        &stats.chaos,
        summary.avg_chaos,
        100.0,
        bad_stat_color(summary.avg_chaos),
        trend_marker(summary.trend_chaos),
    );
    y = theme::stat_bar(
        c,
        y,
        &stats.danger,
        summary.avg_danger,
        100.0,
        bad_stat_color(summary.avg_danger),
        trend_marker(summary.trend_danger),
    );
    theme::stat_bar(
        c,
        y,
        &stats.magic,
        summary.avg_magic,
        100.0,
        good_stat_color(summary.avg_magic),
        trend_marker(summary.trend_magic),
    );
}

fn draw_present_age_card(ctx: &UiContext<'_>, rect: Rect) {
    let strings = &ctx.data.strings;
    let c = theme::sub_card(rect, &strings.eras.current_title);
    let era = &ctx.world.era;
    let era_balance = &ctx.data.balance.era;
    let mut y = c.y + 14.0;

    draw_ui_text_ex(
        &fill(
            &strings.eras.era_line,
            &[
                ("number", era.number.to_string()),
                ("name", era.name.clone()),
            ],
        ),
        c.x,
        y,
        TextStyle::new(15.0, dark::TEXT).params(),
    );
    y += 22.0;

    let breaking = era.pressure >= era_balance.breaking_threshold;
    meter(
        Rect::new(c.x, y, c.w, 20.0),
        era.pressure,
        era_balance.breaking_threshold,
        bad_stat_color(era.pressure / era_balance.breaking_threshold * 100.0),
        Some(&fill(
            &strings.eras.pressure,
            &[("pressure", format!("{:.0}", era.pressure))],
        )),
    );
    y += 32.0;
    draw_ui_text_ex(
        if breaking {
            &strings.eras.breaking
        } else {
            &strings.eras.holding
        },
        c.x,
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
    y += 22.0;

    // Portents: which age the pressure builds toward, and how the pantheon stirs.
    draw_ui_text_ex(
        &fill(
            &strings.eras.trending,
            &[("trigger", era.dominant_trigger.label().to_owned())],
        ),
        c.x,
        y,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );
    y += 20.0;
    draw_ui_text_ex(
        &heavens_line(ctx),
        c.x,
        y,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );
}

fn draw_actions_card(ctx: &UiContext<'_>, rect: Rect, actions: &mut Vec<UiAction>) {
    let ui = &ctx.data.strings.ui;
    let c = theme::sub_card(rect, &ctx.data.strings.panels.divine_actions);
    let gap = 12.0;
    let bw = (c.w - gap) / 2.0;
    let bh = 40.0;
    let col2 = c.x + bw + gap;
    let y0 = c.y + 8.0;
    let y1 = y0 + bh + gap;

    // Each verb jumps to the screen that owns it — only enabled once the deity's
    // Standing has revealed that screen (GDD 5.9), so the row teaches progression.
    action_button(
        Rect::new(c.x, y0, bw, bh),
        &ui.action_boon,
        Screen::Regions,
        ButtonTone::Primary,
        ctx,
        actions,
    );
    action_button(
        Rect::new(col2, y0, bw, bh),
        &ui.action_hinder,
        Screen::Regions,
        ButtonTone::Danger,
        ctx,
        actions,
    );
    action_button(
        Rect::new(c.x, y1, bw, bh),
        &ui.action_inspire,
        Screen::DivineTools,
        ButtonTone::Primary,
        ctx,
        actions,
    );
    action_button(
        Rect::new(col2, y1, bw, bh),
        &ui.action_observe,
        Screen::Betting,
        ButtonTone::Secondary,
        ctx,
        actions,
    );
}

/// A Divine Actions shortcut: enabled only when its target screen is revealed.
fn action_button(
    rect: Rect,
    label: &str,
    target: Screen,
    tone: ButtonTone,
    ctx: &UiContext<'_>,
    actions: &mut Vec<UiAction>,
) {
    let enabled = target.is_revealed(ctx.standing);
    if button(rect, label, enabled, tone, ctx.mouse) {
        actions.push(UiAction::SelectScreen(target));
    }
}

fn draw_events_rail(ctx: &UiContext<'_>, rect: Rect, actions: &mut Vec<UiAction>) {
    draw_titled(rect, &ctx.data.strings.panels.recent_events);
    let content = rect.inset(18.0);
    // Reserve a footer row for the "View Chronicle" link.
    let list_bottom = rect.bottom() - 52.0;
    let mut y = content.y + 40.0;

    if ctx.world.chronicle.is_empty() {
        draw_ui_text_ex(
            &ctx.data.strings.ui.empty_chronicle,
            content.x,
            y,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
    } else {
        for event in ctx.world.chronicle.iter() {
            if y > list_bottom {
                break;
            }
            draw_badge(
                Rect::new(content.x, y - 15.0, 66.0, 20.0),
                &format!("Y{}", event.year),
                Color::new(0.12, 0.15, 0.20, 1.0),
                kind_color(event.kind),
            );
            draw_ui_text_ex(
                &event.message,
                content.x + 78.0,
                y,
                TextStyle::new(15.0, dark::TEXT).params(),
            );
            y += 26.0;
        }
    }

    // Footer: jump to the full event log (always reachable).
    let btn = Rect::new(rect.right() - 176.0, rect.bottom() - 44.0, 160.0, 30.0);
    if button(
        btn,
        &ctx.data.strings.ui.view_chronicle,
        true,
        ButtonTone::Secondary,
        ctx.mouse,
    ) {
        actions.push(UiAction::SelectScreen(Screen::Chronicle));
    }
}

fn draw_heroes_strip(ctx: &UiContext<'_>, rect: Rect, actions: &mut Vec<UiAction>) {
    let strings = &ctx.data.strings;

    // Header row: title + count on the left, a link to the full roster on the right.
    let spotlight = spotlight_heroes(ctx);
    draw_ui_text_ex(
        &format!("{}  ({})", strings.panels.heroes_of_note, spotlight.len()),
        rect.x,
        rect.y + 18.0,
        TextStyle::new(18.0, dark::TEXT_BRIGHT).params(),
    );
    if Screen::Heroes.is_revealed(ctx.standing) {
        let btn = Rect::new(rect.right() - 168.0, rect.y - 2.0, 168.0, 26.0);
        if button(
            btn,
            &strings.ui.view_all_heroes,
            true,
            ButtonTone::Secondary,
            ctx.mouse,
        ) {
            actions.push(UiAction::SelectScreen(Screen::Heroes));
        }
    }

    // Cards.
    let cards = Rect::new(rect.x, rect.y + 28.0, rect.w, rect.h - 28.0);
    if spotlight.is_empty() {
        draw_ui_text_ex(
            &strings.ui.heroes_none,
            cards.x,
            cards.y + 24.0,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
        return;
    }

    let gap = 12.0;
    let cw = (cards.w - gap * (HERO_SPOTLIGHT as f32 - 1.0)) / HERO_SPOTLIGHT as f32;
    for (i, hero) in spotlight.iter().enumerate() {
        let x = cards.x + i as f32 * (cw + gap);
        draw_hero_card(ctx, hero, Rect::new(x, cards.y, cw, cards.h));
    }
}

fn draw_hero_card(ctx: &UiContext<'_>, hero: &Hero, rect: Rect) {
    let accent = role_accent(hero.role);
    draw_surface(
        rect,
        &SurfaceStyle::new(theme::CARD)
            .with_left_accent(3.0, accent)
            .with_border(1.0, theme::BORDER_SOFT),
    );

    // Emblem: a role-tinted disc bearing the calling's initial.
    let ex = rect.x + 26.0;
    let ey = rect.y + rect.h * 0.5;
    draw_circle(ex, ey, 16.0, Color::new(accent.r, accent.g, accent.b, 0.22));
    draw_circle_lines(ex, ey, 16.0, 1.5, accent);
    let initial = hero.role.label().chars().next().unwrap_or('?').to_string();
    let dims = measure_text(&initial, None, 18, 1.0);
    draw_text(
        &initial,
        ex - dims.width * 0.5,
        ey + dims.height * 0.5,
        18.0,
        accent,
    );

    let tx = rect.x + 48.0;
    draw_ui_text_ex(
        &hero.name,
        tx,
        rect.y + 28.0,
        TextStyle::new(15.0, dark::TEXT_BRIGHT).params(),
    );
    draw_ui_text_ex(
        &fill(
            &ctx.data.strings.ui.hero_note_meta,
            &[
                ("level", hero.level.to_string()),
                ("role", hero.role.label().to_owned()),
            ],
        ),
        tx,
        rect.y + 47.0,
        TextStyle::new(12.0, dark::TEXT_DIM).params(),
    );
    let region = ctx
        .world
        .region_name(&hero.region_id)
        .unwrap_or(&hero.region_id);
    draw_ui_text_ex(
        region,
        tx,
        rect.y + 64.0,
        TextStyle::new(12.0, dark::TEXT_DIM).params(),
    );

    // A thin renown bar along the foot: how far toward the next living legend.
    let thresholds = &ctx.data.balance.hero.renown.thresholds;
    let next = thresholds
        .iter()
        .copied()
        .find(|t| *t > hero.renown)
        .unwrap_or_else(|| hero.renown.max(1.0));
    progress_bar(rect.x + 8.0, rect.bottom() - 8.0, rect.w - 16.0, 4.0, hero.renown, next, accent);
}

/// The living heroes with the most renown, most-storied first (GDD 5.4).
fn spotlight_heroes<'a>(ctx: &UiContext<'a>) -> Vec<&'a Hero> {
    let mut heroes: Vec<&'a Hero> = ctx.world.heroes.iter().filter(|h| h.is_alive).collect();
    heroes.sort_by(|a, b| b.renown.total_cmp(&a.renown).then(b.level.cmp(&a.level)));
    heroes.truncate(HERO_SPOTLIGHT);
    heroes
}

/// Count of living heroes who have crossed into legend (top renown title).
fn living_legends(ctx: &UiContext<'_>) -> usize {
    let legend_bar = ctx
        .data
        .balance
        .hero
        .renown
        .thresholds
        .last()
        .copied()
        .unwrap_or(f32::INFINITY);
    ctx.world
        .heroes
        .iter()
        .filter(|h| h.is_alive && h.renown >= legend_bar)
        .count()
}

/// The dashboard's "heavens" portent: the most-roused deity and its mood, or a
/// calm line when the pantheon is dormant. Reads the same tiers the pantheon
/// panel uses, so the two agree.
fn heavens_line(ctx: &UiContext<'_>) -> String {
    let strings = &ctx.data.strings;
    let balance = &ctx.data.balance.pantheon;
    let Some(deity) = ctx
        .world
        .pantheon
        .iter()
        .max_by(|a, b| a.pressure.total_cmp(&b.pressure))
    else {
        return strings.ui.heavens_calm.clone();
    };
    let tier = deity.tier(balance);
    if tier == 0 {
        return strings.ui.heavens_calm.clone();
    }
    let d = &strings.divine;
    let mood = match tier {
        1 => &d.mood_stirring,
        2 => &d.mood_roused,
        3 => &d.mood_wrathful,
        _ => &d.mood_ascendant,
    };
    fill(
        &strings.ui.heavens_roused,
        &[("deity", deity.name.clone()), ("mood", mood.clone())],
    )
}

/// A gradient from a golden green (best tenor) through amber to a dark red
/// (worst), so the age's mood reads at a glance.
fn tenor_color(tier: usize, count: usize) -> Color {
    let t = tier as f32 / count.saturating_sub(1).max(1) as f32;
    Color::new(0.3 + 0.6 * t, 0.75 - 0.4 * t, 0.35 - 0.2 * t, 1.0)
}

/// The signature colour for each hero calling — the emblem tint and card accent.
fn role_accent(role: HeroRole) -> Color {
    match role {
        HeroRole::Warrior => Color::new(0.82, 0.44, 0.36, 1.0),
        HeroRole::Mage => Color::new(0.42, 0.58, 0.94, 1.0),
        HeroRole::Scholar => Color::new(0.36, 0.72, 0.68, 1.0),
        HeroRole::Ranger => Color::new(0.46, 0.74, 0.44, 1.0),
        HeroRole::Merchant => theme::GOLD,
        HeroRole::Cleric => Color::new(0.68, 0.52, 0.88, 1.0),
    }
}

fn kind_color(kind: EventKind) -> Color {
    match kind {
        EventKind::Divine => theme::BLUE,
        EventKind::Region => dark::WARNING,
        EventKind::Hero => Color::new(0.7, 0.55, 0.9, 1.0),
        EventKind::System => dark::POSITIVE,
    }
}
