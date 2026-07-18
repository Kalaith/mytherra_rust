//! Heroes screen: the roster of heroes across the world, their vocations,
//! levels, and lifespans (GDD 10 "Heroes & Champions"). Read-only for now;
//! champion cultivation actions arrive in a later iteration.

use crate::data::fill;
use crate::ui::widgets::good_stat_color;
use crate::ui::{content_rect, UiContext};
use crate::world::Hero;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt};

pub fn draw(ctx: &UiContext<'_>) {
    let strings = &ctx.data.strings.heroes;
    let rect = content_rect();
    let style = SurfaceStyle::new(Color::new(0.07, 0.075, 0.095, 0.96))
        .with_border(1.0, Color::new(0.38, 0.45, 0.58, 0.5))
        .with_header(42.0, Color::new(0.1, 0.115, 0.145, 1.0))
        .with_header_divider(1.0, Color::new(0.38, 0.45, 0.58, 0.4));
    draw_surface_with_title(
        rect,
        Some(&strings.panel),
        &style,
        TextStyle::new(20.0, dark::TEXT),
    );

    let content = rect.inset(18.0);
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
        content.y + 30.0,
        TextStyle::new(15.0, dark::TEXT_DIM).params(),
    );

    if ctx.world.heroes.is_empty() {
        draw_ui_text_ex(
            &strings.empty,
            content.x,
            content.y + 64.0,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
        return;
    }

    // Two columns of hero cards.
    let top = content.y + 48.0;
    let col_gap = 20.0;
    let card_w = (content.w - col_gap) / 2.0;
    let card_h = 92.0;
    let row_gap = 12.0;
    for (index, hero) in ctx.world.heroes.iter().enumerate() {
        let col = (index % 2) as f32;
        let row = (index / 2) as f32;
        let x = content.x + col * (card_w + col_gap);
        let y = top + row * (card_h + row_gap);
        draw_hero_card(ctx, hero, Rect::new(x, y, card_w, card_h));
    }
}

fn draw_hero_card(ctx: &UiContext<'_>, hero: &Hero, rect: Rect) {
    let strings = &ctx.data.strings.heroes;
    let accent = if hero.is_alive {
        good_stat_color(hero.level as f32 * 4.0)
    } else {
        dark::TEXT_DIM
    };
    let style = SurfaceStyle::new(Color::new(0.09, 0.1, 0.13, 1.0))
        .with_left_accent(4.0, accent)
        .with_border(1.0, Color::new(0.4, 0.46, 0.58, 0.35));
    draw_surface(rect, &style);

    let name_color = if hero.is_alive {
        dark::TEXT_BRIGHT
    } else {
        dark::TEXT_DIM
    };
    draw_ui_text_ex(
        &hero.name,
        rect.x + 16.0,
        rect.y + 28.0,
        TextStyle::new(19.0, name_color).params(),
    );
    let region = ctx
        .world
        .region_name(&hero.region_id)
        .unwrap_or(&hero.region_id);
    draw_ui_text_ex(
        &format!("{}  ·  {}", hero.role.label(), region),
        rect.x + 16.0,
        rect.y + 50.0,
        TextStyle::new(14.0, dark::TEXT_DIM).params(),
    );

    // Level badge + alive/fallen state, right-aligned.
    let status = if hero.is_alive {
        &strings.alive
    } else {
        &strings.fallen
    };
    draw_badge(
        Rect::new(rect.right() - 96.0, rect.y + 14.0, 80.0, 24.0),
        &fill(&strings.level, &[("level", hero.level.to_string())]),
        Color::new(0.16, 0.2, 0.28, 1.0),
        dark::TEXT,
    );
    draw_badge(
        Rect::new(rect.right() - 96.0, rect.y + 44.0, 80.0, 22.0),
        status,
        Color::new(0.14, 0.16, 0.2, 1.0),
        accent,
    );

    // Lifespan meter (age vs. life expectancy).
    let life = hero.life_expectancy(&ctx.data.balance.hero);
    let label = fill(
        &strings.life,
        &[
            ("age", hero.age.to_string()),
            ("life", format!("{life:.0}")),
        ],
    );
    meter(
        Rect::new(rect.x + 16.0, rect.bottom() - 24.0, rect.w - 128.0, 16.0),
        hero.age as f32,
        life,
        life_color(hero.age as f32, life),
        Some(&label),
    );
}

/// Green when young, warning as the hero nears life expectancy, red past it.
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
