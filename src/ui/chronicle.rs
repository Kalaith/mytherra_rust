//! Event Log (GDD 10): the full world chronicle, newest first, filterable by
//! event kind. A pure view over `world.chronicle` — the filter chips emit
//! `SetChronicleFilter` intents rather than mutating any state.

use crate::data::fill;
use crate::ui::widgets::{button, draw_titled, page_controls, paginate};
use crate::ui::{content_rect, UiAction, UiContext};
use crate::world::{EventKind, WorldEvent};
use macroquad::prelude::*;
use macroquad_toolkit::prelude::*;
use macroquad_toolkit::ui::{draw_ui_text_ex, RectExt};

pub fn draw(ctx: &UiContext<'_>, actions: &mut Vec<UiAction>) {
    let area = content_rect();
    draw_titled(area, &ctx.data.strings.event_log.panel);
    let content = area.inset(18.0);

    let active = ctx.chronicle_filter;
    draw_filter_chips(ctx, content, active, actions);

    // Filter 0 means "all"; 1..=5 map onto EventKind::ALL.
    let kind = active
        .checked_sub(1)
        .and_then(|i| EventKind::ALL.get(i).copied());
    let filter_name = filter_name(ctx, kind);

    let total = ctx.world.chronicle.iter_newest().count();
    let events: Vec<&WorldEvent> = ctx
        .world
        .chronicle
        .iter_newest()
        .filter(|e| match kind {
            Some(k) => e.kind == k,
            None => true,
        })
        .collect();

    let strings = &ctx.data.strings.event_log;
    let list_top = content.y + 82.0;
    let list_start = list_top + 24.0;
    let row_h = 26.0;
    // Fill the panel with as many rows as fit, then page the rest so the whole
    // chronicle is reachable rather than silently truncated at the fold.
    let page_size = (((content.bottom() - list_start) / row_h).floor() as usize).max(1);
    let (page, start, end, total_pages) = paginate(events.len(), page_size, ctx.chronicle_page);

    draw_ui_text_ex(
        &fill(
            &strings.count_line,
            &[
                ("shown", (end - start).to_string()),
                ("total", total.to_string()),
                ("filter", filter_name.clone()),
            ],
        ),
        content.x,
        list_top - 12.0,
        TextStyle::new(14.0, dark::TEXT_DIM).params(),
    );

    if events.is_empty() {
        draw_ui_text_ex(
            &fill(&strings.empty_filtered, &[("filter", filter_name)]),
            content.x,
            list_start,
            TextStyle::new(15.0, dark::TEXT_DIM).params(),
        );
        return;
    }

    if total_pages > 1 {
        let row = Rect::new(content.x, list_top - 34.0, content.w, 26.0);
        let label = fill(
            &strings.page_label,
            &[
                ("page", (page + 1).to_string()),
                ("pages", total_pages.to_string()),
            ],
        );
        if let Some(target) = page_controls(
            row,
            page,
            total_pages,
            &strings.prev_page,
            &strings.next_page,
            &label,
            ctx.mouse,
        ) {
            actions.push(UiAction::SetChroniclePage(target));
        }
    }

    let mut y = list_start;
    for event in &events[start..end] {
        let color = kind_color(event.kind);
        draw_badge(
            Rect::new(content.x, y - 15.0, 66.0, 20.0),
            &format!("Y{}", event.year),
            Color::new(0.14, 0.16, 0.2, 1.0),
            color,
        );
        draw_badge(
            Rect::new(content.x + 74.0, y - 15.0, 82.0, 20.0),
            event.kind.label(),
            Color::new(0.1, 0.11, 0.14, 1.0),
            color,
        );
        draw_ui_text_ex(
            &event.message,
            content.x + 168.0,
            y,
            TextStyle::new(15.0, dark::TEXT).params(),
        );
        y += row_h;
    }
}

fn draw_filter_chips(
    ctx: &UiContext<'_>,
    content: Rect,
    active: usize,
    actions: &mut Vec<UiAction>,
) {
    draw_ui_text_ex(
        &ctx.data.strings.event_log.filter_label,
        content.x,
        content.y + 30.0,
        TextStyle::new(15.0, dark::TEXT_DIM).params(),
    );

    let mut labels: Vec<&str> = vec![ctx.data.strings.event_log.filter_all.as_str()];
    labels.extend(EventKind::ALL.iter().map(|k| k.label()));

    let chip_w = 104.0;
    let gap = 8.0;
    let x0 = content.x + 64.0;
    let y = content.y + 14.0;
    for (index, label) in labels.iter().enumerate() {
        let rect = Rect::new(x0 + index as f32 * (chip_w + gap), y, chip_w, 30.0);
        let tone = if index == active {
            ButtonTone::Primary
        } else {
            ButtonTone::Secondary
        };
        if button(rect, label, true, tone, ctx.mouse) {
            actions.push(UiAction::SetChronicleFilter(index));
        }
    }
}

fn filter_name(ctx: &UiContext<'_>, kind: Option<EventKind>) -> String {
    match kind {
        Some(k) => k.label().to_string(),
        None => ctx.data.strings.event_log.filter_all.clone(),
    }
}

fn kind_color(kind: EventKind) -> Color {
    match kind {
        EventKind::Tick => dark::TEXT_DIM,
        EventKind::Divine => dark::ACCENT,
        EventKind::Region => dark::WARNING,
        EventKind::Hero => Color::new(0.7, 0.55, 0.9, 1.0),
        EventKind::System => dark::POSITIVE,
    }
}
