//! The selected region's detail panel: stat meters, divine action cards,
//! genesis outlook, looming aftermaths, and holdings (GDD 10). Split from
//! `regions` to keep each file focused.

use crate::data::{fill, RegionActionDef};
use crate::ui::widgets::{bad_stat_color, button, draw_titled, good_stat_color, trend_marker};
use crate::ui::{UiAction, UiContext};
use crate::world::{Region, RegionStatus};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt};

use super::selected_index;

pub(super) fn draw_region_detail(ctx: &UiContext<'_>, rect: Rect, actions: &mut Vec<UiAction>) {
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
        region.prosperity - region.prev.prosperity,
    );
    ly = stat(
        left_x,
        ly,
        col_w,
        &stats.chaos,
        region.chaos,
        bad_stat_color(region.chaos),
        region.chaos - region.prev.chaos,
    );
    ly = stat(
        left_x,
        ly,
        col_w,
        &stats.danger,
        region.danger,
        bad_stat_color(region.danger),
        region.danger - region.prev.danger,
    );
    let mut ry = y;
    ry = stat(
        right_x,
        ry,
        col_w,
        &stats.magic,
        region.magic_affinity,
        good_stat_color(region.magic_affinity),
        region.magic_affinity - region.prev.magic_affinity,
    );
    ry = stat(
        right_x,
        ry,
        col_w,
        &stats.culture,
        region.cultural_influence,
        good_stat_color(region.cultural_influence),
        0.0,
    );
    ry = stat(
        right_x,
        ry,
        col_w,
        &stats.resonance,
        region.divine_resonance,
        good_stat_color(region.divine_resonance),
        0.0,
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

    // Secession pressure, shown only once strife is genuinely brewing so calm
    // regions stay uncluttered (GDD 5.2 — surface cause and effect).
    let genesis = &ctx.data.balance.genesis;
    if region.strife >= 1.0 {
        let gtext = &strings.genesis;
        let pct = (region.strife / genesis.fracture_threshold * 100.0).round() as i32;
        draw_ui_text_ex(
            &fill(
                &gtext.strife_line,
                &[
                    (
                        "stage",
                        strife_stage(region.strife, genesis, gtext).to_owned(),
                    ),
                    ("pct", pct.to_string()),
                ],
            ),
            content.x,
            y,
            TextStyle::new(14.0, bad_stat_color(region.strife.min(100.0))).params(),
        );
        y += 22.0;
    }

    // Military might and the region's genesis outlook (GDD 5.2 — surface why the
    // map reshapes: which regions can conquer, expand, or be swallowed). Effective
    // might folds in any War-artifact empowerment, so the shown number matches
    // what conquest actually weighs.
    let gtext = &strings.genesis;
    let conquest = &ctx.data.balance.conquest;
    let war_might: f32 = ctx
        .world
        .artifacts
        .iter()
        .filter(|a| a.focus == crate::data::ArtifactFocus::War && a.region_id == region.id)
        .map(|a| a.power as f32 * conquest.artifact_war_might)
        .sum();
    draw_ui_text_ex(
        &fill(
            &gtext.might_line,
            &[(
                "might",
                format!("{:.0}", region.might(conquest) + war_might),
            )],
        ),
        content.x,
        y,
        TextStyle::new(14.0, dark::TEXT_DIM).params(),
    );
    y += 22.0;
    if let Some((text, color)) = genesis_outlook(ctx, region) {
        draw_ui_text_ex(&text, content.x, y, TextStyle::new(14.0, color).params());
        y += 22.0;
    }

    // Scheduled consequences bound for this region, surfaced so the player can
    // foresee them (GDD 5.6). A coming harvest (bloom) is foretold apart from a
    // coming scar (blight/backlash) — the two shouldn't read alike.
    let (boons, banes): (usize, usize) = ctx
        .world
        .pending_consequences
        .iter()
        .filter(|c| c.region_id == region.id)
        .fold((0, 0), |(b, m), c| {
            if c.effect.is_boon() {
                (b + 1, m)
            } else {
                (b, m + 1)
            }
        });
    if banes > 0 {
        draw_ui_text_ex(
            &fill(&strings.ui.aftermath_looms, &[("count", banes.to_string())]),
            content.x,
            y,
            TextStyle::new(14.0, dark::NEGATIVE).params(),
        );
        y += 22.0;
    }
    if boons > 0 {
        draw_ui_text_ex(
            &fill(&strings.ui.boon_ripens, &[("count", boons.to_string())]),
            content.x,
            y,
            TextStyle::new(14.0, dark::POSITIVE).params(),
        );
        y += 22.0;
    }

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
            Rect::new(content.x, y, content.w, 48.0),
            actions,
        );
        y += 54.0;
    }

    // Holdings: the region's settlements and resource nodes.
    y += 4.0;
    draw_ui_text_ex(
        &strings.ui.holdings,
        content.x,
        y,
        TextStyle::new(16.0, dark::TEXT_BRIGHT).params(),
    );
    y += 12.0;

    let town_total = ctx
        .world
        .settlements
        .iter()
        .filter(|s| s.region_id == region.id)
        .count();
    let tier_thresholds = &ctx.data.balance.settlement.tier_thresholds;
    let tier_names = &ctx.data.strings.ui.settlement_tiers;
    let towns: Vec<String> = ctx
        .world
        .settlements
        .iter()
        .filter(|s| s.region_id == region.id)
        .take(3)
        .map(|s| {
            let tier = tier_names
                .get(s.tier(tier_thresholds))
                .map(String::as_str)
                .unwrap_or_default();
            format!("{} ({}, {:.1}k)", s.name, tier, s.population / 1000.0)
        })
        .collect();
    let node_total = ctx
        .world
        .resource_nodes
        .iter()
        .filter(|n| n.region_id == region.id)
        .count();
    let nodes: Vec<String> = ctx
        .world
        .resource_nodes
        .iter()
        .filter(|n| n.region_id == region.id)
        .take(3)
        .map(|n| format!("{} ({})", n.name, n.status.label()))
        .collect();
    let mark_total = ctx
        .world
        .landmarks
        .iter()
        .filter(|l| l.region_id == region.id)
        .count();
    let marks: Vec<String> = ctx
        .world
        .landmarks
        .iter()
        .filter(|l| l.region_id == region.id)
        .take(3)
        .map(|l| l.name.clone())
        .collect();
    let trades: Vec<String> = ctx
        .world
        .trade_routes
        .iter()
        .filter_map(|t| t.other(&region.id))
        .map(|id| ctx.world.region_name(id).unwrap_or(id).to_owned())
        .collect();
    let in_region = |b: &&crate::world::Building| {
        ctx.world
            .settlements
            .iter()
            .any(|s| s.id == b.settlement_id && s.region_id == region.id)
    };
    let build_total = ctx.world.buildings.iter().filter(in_region).count();
    let builds: Vec<String> = ctx
        .world
        .buildings
        .iter()
        .filter(in_region)
        .take(3)
        .map(|b| b.name.clone())
        .collect();

    let ui = &strings.ui;
    // A holdings line names up to three, then notes how many more it holds, so a
    // region grown thick with towns and wonders shows its full weight (GDD 10).
    let more = |items: &[String], total: usize| -> String {
        let mut s = items.join(",  ");
        if total > items.len() {
            s.push_str(&fill(
                &ui.holdings_more,
                &[("count", (total - items.len()).to_string())],
            ));
        }
        s
    };
    let lines: Vec<String> = [
        (!towns.is_empty())
            .then(|| fill(&ui.settlements_line, &[("list", more(&towns, town_total))])),
        (!nodes.is_empty())
            .then(|| fill(&ui.resources_line, &[("list", more(&nodes, node_total))])),
        (!marks.is_empty())
            .then(|| fill(&ui.landmarks_line, &[("list", more(&marks, mark_total))])),
        (!trades.is_empty())
            .then(|| fill(&ui.trade_line, &[("list", more(&trades, trades.len()))])),
        (!builds.is_empty())
            .then(|| fill(&ui.buildings_line, &[("list", more(&builds, build_total))])),
    ]
    .into_iter()
    .flatten()
    .collect();

    if lines.is_empty() {
        draw_ui_text_ex(
            &ui.no_holdings,
            content.x,
            y + 14.0,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
        );
        return;
    }
    // Draw only the holdings lines that fit above the panel's bottom edge, so a
    // region weighed down by looming aftermaths never spills its holdings over
    // the status bar below (GDD 10).
    for line in lines {
        if y + 18.0 > content.bottom() {
            break;
        }
        draw_ui_text_ex(
            &line,
            content.x,
            y + 13.0,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
        );
        y += 18.0;
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

/// The region's genesis outlook line + colour, if it has one: a thriving land
/// ready to spawn a frontier, or a crisis region's conquest exposure. Reads the
/// same balance the sim's genesis paths use, so the cue matches the mechanic.
fn genesis_outlook(ctx: &UiContext<'_>, region: &Region) -> Option<(String, Color)> {
    let g = &ctx.data.strings.genesis;
    let frontier = &ctx.data.balance.frontier;
    if region.status == RegionStatus::Thriving
        && region.population >= frontier.parent_min_population
    {
        return Some((g.outlook_frontier.clone(), dark::POSITIVE));
    }
    if region.status.is_crisis() {
        let conquest = &ctx.data.balance.conquest;
        // A Protection ward turns back conquest outright — surface it first.
        let warded = ctx.world.artifacts.iter().any(|a| {
            a.focus == crate::data::ArtifactFocus::Protection
                && a.region_id == region.id
                && a.power >= conquest.shield_min_power
        });
        if warded {
            return Some((g.outlook_warded.clone(), dark::ACCENT));
        }
        let defended = ctx.world.heroes.iter().any(|h| {
            h.is_alive && h.region_id == region.id && h.level >= conquest.defender_min_level
        });
        return Some(if defended {
            (g.outlook_defended.clone(), dark::ACCENT)
        } else {
            (g.outlook_vulnerable.clone(), dark::NEGATIVE)
        });
    }
    None
}

/// How close a region is to fracturing, as an escalating descriptor (view-only).
fn strife_stage<'a>(
    strife: f32,
    balance: &crate::data::GenesisBalance,
    text: &'a crate::data::strings::GenesisText,
) -> &'a str {
    let ratio = strife / balance.fracture_threshold;
    if ratio >= 0.8 {
        &text.strife_breaking
    } else if ratio >= 0.4 {
        &text.strife_seething
    } else {
        &text.strife_simmering
    }
}

fn stat(x: f32, y: f32, w: f32, label: &str, value: f32, color: Color, trend: f32) -> f32 {
    meter(
        Rect::new(x, y, w, 20.0),
        value,
        100.0,
        color,
        Some(&format!("{label} {value:.0}{}", trend_marker(trend))),
    );
    y + 28.0
}

fn badge(x: f32, y: f32, w: f32, label: &str) -> f32 {
    let rect = Rect::new(x, y, w, 26.0);
    draw_badge(rect, label, Color::new(0.14, 0.17, 0.22, 1.0), dark::TEXT);
    rect.right() + 10.0
}
