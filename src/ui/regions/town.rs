//! The town browser: a drill-in from a region's holdings that lists the region's
//! towns and shows the selected one's detail (GDD 10). Split from `detail` to
//! keep each file focused. Pure view — clicks return `UiAction` intents.

use crate::data::fill;
use crate::ui::widgets::{button, draw_titled, good_stat_color};
use crate::ui::{UiAction, UiContext};
use crate::world::Region;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt};

/// Draw the town browser over the region-detail panel: the left column lists
/// every town in the region (clickable to switch), the right column shows the
/// chosen town's tier, population, prosperity, and the works raised in it. A Back
/// button closes it.
pub(super) fn draw_town_browser(
    ctx: &UiContext<'_>,
    rect: Rect,
    region: &Region,
    town_id: &str,
    actions: &mut Vec<UiAction>,
) {
    let ui = &ctx.data.strings.ui;
    draw_titled(
        rect,
        &fill(&ui.town_browser_title, &[("region", region.name.clone())]),
    );
    let content = rect.inset(18.0);

    let back = Rect::new(content.x, content.y + 28.0, 96.0, 30.0);
    if button(back, &ui.town_close, true, ButtonTone::Secondary, ctx.mouse) {
        actions.push(UiAction::CloseTown);
    }

    let thresholds = &ctx.data.balance.settlement.tier_thresholds;
    let tier_of = |s: &crate::world::Settlement| {
        ui.settlement_tiers
            .get(s.tier(thresholds))
            .map(String::as_str)
            .unwrap_or_default()
            .to_owned()
    };

    // Two columns: town list (left), selected town's detail (right).
    let list_w = (content.w * 0.4).min(300.0);
    let detail_x = content.x + list_w + 22.0;
    let detail_w = content.right() - detail_x;

    // Left: a clickable row per town in the region.
    let mut ly = content.y + 74.0;
    for s in ctx
        .world
        .settlements
        .iter()
        .filter(|s| s.region_id == region.id)
    {
        if ly + 40.0 > content.bottom() {
            break;
        }
        let row = Rect::new(content.x, ly, list_w, 40.0);
        let is_sel = s.id == town_id;
        let hovered = row.contains_point(ctx.mouse);
        let fill_color = if is_sel {
            Color::new(0.15, 0.19, 0.26, 1.0)
        } else if hovered {
            Color::new(0.12, 0.14, 0.18, 1.0)
        } else {
            Color::new(0.09, 0.1, 0.13, 1.0)
        };
        draw_surface(
            row,
            &SurfaceStyle::new(fill_color)
                .with_left_accent(4.0, good_stat_color(s.prosperity))
                .with_border(1.0, Color::new(0.4, 0.46, 0.58, 0.35)),
        );
        draw_ui_text_ex(
            &s.name,
            row.x + 12.0,
            row.y + 17.0,
            TextStyle::new(
                15.0,
                if is_sel {
                    dark::TEXT_BRIGHT
                } else {
                    dark::TEXT
                },
            )
            .params(),
        );
        draw_ui_text_ex(
            &format!("{}  ·  {:.1}k", tier_of(s), s.population / 1000.0),
            row.x + 12.0,
            row.y + 33.0,
            TextStyle::new(12.0, dark::TEXT_DIM).params(),
        );
        if hovered && !is_sel && is_mouse_button_released(MouseButton::Left) {
            actions.push(UiAction::SelectTown(s.id.clone()));
        }
        ly += 46.0;
    }

    // Right: the chosen town's detail.
    let Some(town) = ctx.world.settlements.iter().find(|s| s.id == town_id) else {
        return;
    };
    let mut y = content.y + 74.0;
    draw_ui_text_ex(
        &town.name,
        detail_x,
        y,
        TextStyle::new(20.0, dark::TEXT_BRIGHT).params(),
    );
    y += 26.0;
    draw_ui_text_ex(
        &fill(&ui.town_tier, &[("tier", tier_of(town))]),
        detail_x,
        y,
        TextStyle::new(15.0, dark::ACCENT).params(),
    );
    y += 24.0;
    draw_ui_text_ex(
        &fill(
            &ui.town_population,
            &[("pop", format!("{:.0}", town.population))],
        ),
        detail_x,
        y,
        TextStyle::new(14.0, dark::TEXT_DIM).params(),
    );
    y += 24.0;
    meter(
        Rect::new(detail_x, y, detail_w, 20.0),
        town.prosperity,
        100.0,
        good_stat_color(town.prosperity),
        Some(&fill(
            &ui.town_prosperity,
            &[("prosperity", format!("{:.0}", town.prosperity))],
        )),
    );
    y += 36.0;
    draw_ui_text_ex(
        &ui.town_buildings,
        detail_x,
        y,
        TextStyle::new(16.0, dark::TEXT_BRIGHT).params(),
    );
    y += 22.0;
    let mut any_work = false;
    for b in ctx
        .world
        .buildings
        .iter()
        .filter(|b| b.settlement_id == town.id)
    {
        if y + 18.0 > content.bottom() {
            break;
        }
        any_work = true;
        // The building type's own name reads cleaner than the "{town} {type}"
        // instance name, so a Forge shows as "Forge", not "Aldervale Forge".
        let name = ctx
            .data
            .building_types
            .get(&b.type_id)
            .map(|t| t.name.clone())
            .unwrap_or_else(|| b.name.clone());
        draw_ui_text_ex(
            &format!("-  {name}"),
            detail_x,
            y + 13.0,
            TextStyle::new(13.0, dark::TEXT).params(),
        );
        y += 18.0;
    }
    if !any_work {
        draw_ui_text_ex(
            &ui.town_no_buildings,
            detail_x,
            y + 13.0,
            TextStyle::new(13.0, dark::TEXT_DIM).params(),
        );
    }
}
