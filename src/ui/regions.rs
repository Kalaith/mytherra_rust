//! Regions screen: the region roster on the left, with the selected region's
//! detail + divine actions on the right (`detail` submodule) (GDD 10).

use crate::data::fill;
use crate::ui::widgets::{draw_titled, good_stat_color, page_controls, paginate};
use crate::ui::{content_rect, UiAction, UiContext};
use crate::world::Region;
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt};

mod detail;

pub fn draw(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let area = content_rect();
    let list = Rect::new(area.x, area.y, 360.0, area.h);
    let detail = Rect::new(
        list.right() + 16.0,
        area.y,
        area.right() - list.right() - 16.0,
        area.h,
    );

    draw_region_list(ctx, list, actions);
    detail::draw_region_detail(ctx, detail, actions);
}

fn draw_region_list(ctx: &UiContext<'_>, rect: Rect, actions: &mut Vec<UiAction>) {
    draw_titled(rect, &ctx.data.strings.panels.regions);
    let content = rect.inset(16.0);
    let selected = selected_index(ctx);

    // Page the roster: region genesis grows the list past the panel, so drawing
    // every card would spill off the bottom (GDD 5.2 <-> 10).
    let list_start = content.y + 34.0;
    let stride = 78.0;
    let pager_row = Rect::new(content.x, content.bottom() - 26.0, content.w, 24.0);
    let page_size = (((pager_row.y - 6.0 - list_start) / stride).floor() as usize).max(1);
    let (page, start, end, total_pages) =
        paginate(ctx.world.regions.len(), page_size, ctx.region_page);

    let mut y = list_start;
    // `index` stays the true world index so selection is unaffected by paging.
    for (index, region) in ctx.world.regions.iter().enumerate().take(end).skip(start) {
        let card = Rect::new(content.x, y, content.w, 68.0);
        let hovered = card.contains_point(ctx.mouse);
        let is_selected = index == selected;
        let fill_color = if is_selected {
            Color::new(0.15, 0.19, 0.26, 1.0)
        } else if hovered {
            Color::new(0.12, 0.14, 0.18, 1.0)
        } else {
            Color::new(0.09, 0.1, 0.13, 1.0)
        };
        let style = SurfaceStyle::new(fill_color)
            .with_left_accent(4.0, status_color(region))
            .with_border(1.0, Color::new(0.4, 0.46, 0.58, 0.35));
        draw_surface(card, &style);

        draw_ui_text_ex(
            &region.name,
            card.x + 16.0,
            card.y + 26.0,
            TextStyle::new(18.0, dark::TEXT_BRIGHT).params(),
        );
        draw_ui_text_ex(
            &fill(
                &ctx.data.strings.ui.region_subtitle,
                &[
                    ("status", region.status.label().to_owned()),
                    ("culture", region.culture.label().to_owned()),
                ],
            ),
            card.x + 16.0,
            card.y + 48.0,
            TextStyle::new(14.0, dark::TEXT_DIM).params(),
        );
        meter(
            Rect::new(card.right() - 108.0, card.y + 22.0, 92.0, 16.0),
            region.prosperity,
            100.0,
            good_stat_color(region.prosperity),
            None,
        );

        if hovered && is_mouse_button_released(MouseButton::Left) {
            actions.push(UiAction::SelectRegion(index));
        }
        y += stride;
    }

    if total_pages > 1 {
        let ui = &ctx.data.strings.ui;
        let label = fill(
            &ui.page_label,
            &[
                ("page", (page + 1).to_string()),
                ("pages", total_pages.to_string()),
            ],
        );
        if let Some(target) = page_controls(
            pager_row,
            page,
            total_pages,
            &ui.page_prev,
            &ui.page_next,
            &label,
            ctx.mouse,
        ) {
            actions.push(UiAction::SetRegionPage(target));
        }
    }
}

fn selected_index(ctx: &UiContext<'_>) -> usize {
    ctx.selected_region
        .min(ctx.world.regions.len().saturating_sub(1))
}

fn status_color(region: &Region) -> Color {
    use crate::world::RegionStatus::*;
    match region.status {
        Thriving => dark::POSITIVE,
        Prospering => Color::new(0.4, 0.7, 0.5, 1.0),
        Peaceful => dark::ACCENT,
        Unrest => dark::WARNING,
        Struggling => Color::new(0.85, 0.5, 0.3, 1.0),
        WarTorn => dark::NEGATIVE,
    }
}
