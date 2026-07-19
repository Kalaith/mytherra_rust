//! Heroes screen: cultivated champions on the left, the full hero roster with
//! designation on the right (GDD 10 "Heroes & Champions").

use crate::data::fill;
use crate::ui::widgets::{button, good_stat_color};
use crate::ui::{content_rect, UiAction, UiContext};
use crate::world::{Champion, Hero};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt};

pub fn draw(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let area = content_rect();
    let left = Rect::new(area.x, area.y, 372.0, area.h);
    let right = Rect::new(
        left.right() + 16.0,
        area.y,
        area.right() - left.right() - 16.0,
        area.h,
    );

    draw_champions_panel(ctx, left, actions);
    draw_roster_panel(ctx, right, actions);
}

fn draw_champions_panel(ctx: &UiContext<'_>, rect: Rect, actions: &mut Vec<UiAction>) {
    let strings = &ctx.data.strings.heroes;
    let max = ctx.data.balance.champion.max_roster;
    let title = fill(
        &strings.champions_title,
        &[
            ("count", ctx.player.champions.len().to_string()),
            ("max", max.to_string()),
        ],
    );
    draw_titled(rect, &title);
    let content = rect.inset(16.0);

    if ctx.player.champions.is_empty() {
        draw_ui_text_ex(
            &strings.no_champions,
            content.x,
            content.y + 34.0,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
        return;
    }

    let mut y = content.y + 30.0;
    for champion in &ctx.player.champions {
        draw_champion_card(
            ctx,
            champion,
            Rect::new(content.x, y, content.w, 128.0),
            actions,
        );
        y += 140.0;
    }
}

/// Short description of what a champion's focus does on a resolved rivalry,
/// so the cultivation choice is legible (matches the sim in sim/champion.rs).
fn focus_effect<'a>(ctx: &'a UiContext<'_>, focus: crate::data::ChampionFocus) -> &'a str {
    use crate::data::ChampionFocus;
    let strings = &ctx.data.strings.heroes;
    match focus {
        ChampionFocus::Valor => &strings.focus_effect_valor,
        ChampionFocus::Wisdom => &strings.focus_effect_wisdom,
        ChampionFocus::Devotion => &strings.focus_effect_devotion,
    }
}

fn draw_champion_card(
    ctx: &UiContext<'_>,
    champion: &Champion,
    rect: Rect,
    actions: &mut Vec<UiAction>,
) {
    let strings = &ctx.data.strings.heroes;
    let balance = &ctx.data.balance.champion;
    let hero = ctx.world.heroes.iter().find(|h| h.id == champion.hero_id);
    let alive = hero.map(|h| h.is_alive).unwrap_or(false);
    let name = hero.map(|h| h.name.as_str()).unwrap_or(&champion.hero_id);

    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.1, 0.11, 0.15, 1.0))
            .with_left_accent(4.0, if alive { dark::ACCENT } else { dark::TEXT_DIM })
            .with_border(1.0, Color::new(0.4, 0.46, 0.58, 0.35)),
    );
    draw_ui_text_ex(
        name,
        rect.x + 14.0,
        rect.y + 24.0,
        TextStyle::new(18.0, dark::TEXT_BRIGHT).params(),
    );
    draw_ui_text_ex(
        &fill(
            &strings.champion_meta,
            &[
                ("rank", champion.rank.to_string()),
                ("quests", champion.quests.to_string()),
                ("bond", format!("{:.0}", champion.bond)),
            ],
        ),
        rect.x + 14.0,
        rect.y + 46.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );

    // What this focus does when the champion resolves a rivalry.
    draw_ui_text_ex(
        &fill(
            &strings.focus_line,
            &[
                ("focus", champion.focus.label().to_owned()),
                ("effect", focus_effect(ctx, champion.focus).to_owned()),
            ],
        ),
        rect.x + 14.0,
        rect.y + 64.0,
        TextStyle::new(13.0, dark::ACCENT).params(),
    );

    // Quest progress meter.
    meter(
        Rect::new(rect.x + 14.0, rect.y + 76.0, rect.w - 28.0, 14.0),
        champion.quest_progress,
        balance.quest.goal,
        dark::ACCENT,
        Some(&fill(
            &strings.quest,
            &[
                ("progress", format!("{:.0}", champion.quest_progress)),
                ("goal", format!("{:.0}", balance.quest.goal)),
            ],
        )),
    );

    // Focus cycle + cultivate buttons.
    let btn_y = rect.bottom() - 34.0;
    let half = (rect.w - 28.0 - 8.0) / 2.0;
    if button(
        Rect::new(rect.x + 14.0, btn_y, half, 28.0),
        &fill(
            &strings.focus_cycle,
            &[("focus", champion.focus.label().to_owned())],
        ),
        true,
        ButtonTone::Secondary,
        ctx.mouse,
    ) {
        actions.push(UiAction::CycleChampionFocus(champion.hero_id.clone()));
    }
    let cost = champion.cultivate_cost(balance);
    let can = alive && ctx.player.can_afford(cost);
    if button(
        Rect::new(rect.x + 22.0 + half, btn_y, half, 28.0),
        &fill(&strings.cultivate, &[("cost", cost.to_string())]),
        can,
        ButtonTone::Positive,
        ctx.mouse,
    ) {
        actions.push(UiAction::CultivateChampion(champion.hero_id.clone()));
    }
}

fn draw_roster_panel(ctx: &UiContext<'_>, rect: Rect, actions: &mut Vec<UiAction>) {
    let strings = &ctx.data.strings.heroes;
    draw_titled(rect, &strings.roster_label);
    let content = rect.inset(16.0);

    let living = ctx.world.living_heroes();
    draw_ui_text_ex(
        &fill(
            &strings.count,
            &[
                ("alive", living.to_string()),
                ("fallen", (ctx.world.heroes.len() - living).to_string()),
            ],
        ),
        content.x,
        content.y + 24.0,
        TextStyle::new(15.0, dark::TEXT_DIM).params(),
    );

    draw_roster_filter(ctx, content, actions);

    // The roster outgrows the panel (12+ heroes, more born over time), so filter
    // by region, sort the most notable first, and truncate what doesn't fit.
    let region_id: Option<&str> = ctx
        .hero_filter
        .checked_sub(1)
        .and_then(|i| ctx.world.regions.get(i))
        .map(|r| r.id.as_str());
    let mut heroes: Vec<&Hero> = ctx
        .world
        .heroes
        .iter()
        .filter(|h| region_id.is_none_or(|id| h.region_id == id))
        .collect();
    // Living before fallen, then higher level, then stable by id.
    heroes.sort_by(|a, b| {
        b.is_alive
            .cmp(&a.is_alive)
            .then(b.level.cmp(&a.level))
            .then(a.id.cmp(&b.id))
    });

    let roster_full = ctx.player.champions.len() >= ctx.data.balance.champion.max_roster;
    let mut y = content.y + 74.0;
    let mut shown = 0;
    for hero in &heroes {
        // Leave a row's worth of space for the "+N more" note if truncating.
        if y + 66.0 > content.bottom() - 22.0 {
            break;
        }
        draw_hero_card(
            ctx,
            hero,
            Rect::new(content.x, y, content.w, 66.0),
            roster_full,
            actions,
        );
        y += 74.0;
        shown += 1;
    }
    if shown < heroes.len() {
        draw_ui_text_ex(
            &fill(
                &strings.roster_more,
                &[("count", (heroes.len() - shown).to_string())],
            ),
            content.x,
            y + 16.0,
            TextStyle::new(14.0, dark::TEXT_DIM).params(),
        );
    }
}

/// Region filter chips (All + one per region) for the roster.
fn draw_roster_filter(ctx: &UiContext<'_>, content: Rect, actions: &mut Vec<UiAction>) {
    let mut labels: Vec<&str> = vec![ctx.data.strings.heroes.filter_all.as_str()];
    labels.extend(ctx.world.regions.iter().map(|r| r.name.as_str()));

    let gap = 8.0;
    let chip_w = ((content.w - gap * (labels.len() as f32 - 1.0)) / labels.len() as f32).min(168.0);
    let y = content.y + 36.0;
    for (index, label) in labels.iter().enumerate() {
        let rect = Rect::new(content.x + index as f32 * (chip_w + gap), y, chip_w, 30.0);
        let tone = if index == ctx.hero_filter {
            ButtonTone::Primary
        } else {
            ButtonTone::Secondary
        };
        if button(rect, label, true, tone, ctx.mouse) {
            actions.push(UiAction::SetHeroFilter(index));
        }
    }
}

fn draw_hero_card(
    ctx: &UiContext<'_>,
    hero: &Hero,
    rect: Rect,
    roster_full: bool,
    actions: &mut Vec<UiAction>,
) {
    let strings = &ctx.data.strings.heroes;
    let is_champion = ctx.player.is_champion(&hero.id);
    let accent = if hero.is_alive {
        good_stat_color(hero.level as f32 * 4.0)
    } else {
        dark::TEXT_DIM
    };
    draw_surface(
        rect,
        &SurfaceStyle::new(Color::new(0.09, 0.1, 0.13, 1.0))
            .with_left_accent(4.0, accent)
            .with_border(1.0, Color::new(0.4, 0.46, 0.58, 0.35)),
    );

    let name_color = if hero.is_alive {
        dark::TEXT_BRIGHT
    } else {
        dark::TEXT_DIM
    };
    draw_ui_text_ex(
        &hero.name,
        rect.x + 14.0,
        rect.y + 25.0,
        TextStyle::new(18.0, name_color).params(),
    );
    let region = ctx
        .world
        .region_name(&hero.region_id)
        .unwrap_or(&hero.region_id);
    let title = hero.title(
        &strings.renown_titles,
        &ctx.data.balance.hero.renown.thresholds,
    );
    let level_text = fill(&strings.level, &[("level", hero.level.to_string())]);
    let meta = if title.is_empty() {
        fill(
            &strings.untitled_meta,
            &[
                ("role", hero.role.label().to_owned()),
                ("region", region.to_owned()),
                ("level", level_text),
            ],
        )
    } else {
        fill(
            &strings.titled_meta,
            &[
                ("title", title.to_owned()),
                ("role", hero.role.label().to_owned()),
                ("region", region.to_owned()),
                ("level", level_text),
            ],
        )
    };
    draw_ui_text_ex(
        &meta,
        rect.x + 14.0,
        rect.y + 47.0,
        TextStyle::new(13.0, dark::TEXT_DIM).params(),
    );

    // Lifespan meter (middle).
    let life = hero.life_expectancy(&ctx.data.balance.hero);
    meter(
        Rect::new(rect.x + 360.0, rect.y + 14.0, 220.0, 16.0),
        hero.age as f32,
        life,
        life_color(hero.age as f32, life),
        Some(&fill(
            &strings.life,
            &[
                ("age", hero.age.to_string()),
                ("life", format!("{life:.0}")),
            ],
        )),
    );

    // Champion tag or Designate button (right).
    let action_rect = Rect::new(rect.right() - 128.0, rect.y + 16.0, 114.0, 34.0);
    if is_champion {
        draw_badge(
            action_rect,
            &strings.champion_tag,
            Color::new(0.16, 0.2, 0.28, 1.0),
            dark::ACCENT,
        );
    } else {
        let can = hero.is_alive && !roster_full;
        if button(
            action_rect,
            &strings.designate,
            can,
            ButtonTone::Primary,
            ctx.mouse,
        ) {
            actions.push(UiAction::DesignateChampion(hero.id.clone()));
        }
    }
}

fn life_color(age: f32, life: f32) -> Color {
    let ratio = if life > 0.0 { age / life } else { 1.0 };
    if ratio < 0.6 {
        dark::POSITIVE
    } else if ratio < 0.9 {
        dark::WARNING
    } else {
        dark::NEGATIVE
    }
}

fn draw_titled(rect: Rect, title: &str) {
    let style = SurfaceStyle::new(Color::new(0.07, 0.075, 0.095, 0.96))
        .with_border(1.0, Color::new(0.38, 0.45, 0.58, 0.5))
        .with_header(42.0, Color::new(0.1, 0.115, 0.145, 1.0))
        .with_header_divider(1.0, Color::new(0.38, 0.45, 0.58, 0.4));
    draw_surface_with_title(rect, Some(title), &style, TextStyle::new(20.0, dark::TEXT));
}
