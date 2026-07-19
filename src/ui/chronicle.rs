//! Event Log (GDD 10): the full world chronicle, newest first, filterable by
//! event kind. A pure view over `world.chronicle` — the filter chips emit
//! `SetChronicleFilter` intents rather than mutating any state.

use crate::data::fill;
use crate::ui::widgets::{button, draw_titled};
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
        draw_pager(ctx, content, list_top - 34.0, page, total_pages, actions);
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

/// Clamp `requested` to a valid page over `count` items and return
/// `(page, start, end, total_pages)`. `page_size` is floored at 1; an empty list
/// still yields one (empty) page so callers never divide by zero.
fn paginate(count: usize, page_size: usize, requested: usize) -> (usize, usize, usize, usize) {
    let page_size = page_size.max(1);
    let total_pages = count.div_ceil(page_size).max(1);
    let page = requested.min(total_pages - 1);
    let start = page * page_size;
    let end = (start + page_size).min(count);
    (page, start, end, total_pages)
}

/// Newer/Older page controls, right-aligned on the header row. The view passes
/// the exact clamped target page, so a stale stored page can never strand the
/// player (the buttons always step from what's actually shown).
fn draw_pager(
    ctx: &UiContext<'_>,
    content: Rect,
    y: f32,
    page: usize,
    total_pages: usize,
    actions: &mut Vec<UiAction>,
) {
    let strings = &ctx.data.strings.event_log;
    let bw = 84.0;
    let h = 26.0;
    let label_w = 116.0;
    let next = Rect::new(content.right() - bw, y, bw, h);
    let label_x = next.x - 10.0 - label_w;
    let prev = Rect::new(label_x - 10.0 - bw, y, bw, h);

    if button(
        prev,
        &strings.prev_page,
        page > 0,
        ButtonTone::Secondary,
        ctx.mouse,
    ) {
        actions.push(UiAction::SetChroniclePage(page - 1));
    }
    draw_ui_text_ex(
        &fill(
            &strings.page_label,
            &[
                ("page", (page + 1).to_string()),
                ("pages", total_pages.to_string()),
            ],
        ),
        label_x,
        y + 18.0,
        TextStyle::new(14.0, dark::TEXT_DIM).params(),
    );
    if button(
        next,
        &strings.next_page,
        page + 1 < total_pages,
        ButtonTone::Secondary,
        ctx.mouse,
    ) {
        actions.push(UiAction::SetChroniclePage(page + 1));
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

#[cfg(test)]
mod tests {
    use super::paginate;

    #[test]
    fn first_page_starts_at_the_newest() {
        // 30 events, 14 per page -> 3 pages; page 0 shows the first 14.
        assert_eq!(paginate(30, 14, 0), (0, 0, 14, 3));
    }

    #[test]
    fn a_middle_and_last_page_slice_correctly() {
        assert_eq!(paginate(30, 14, 1), (1, 14, 28, 3));
        // The short final page stops at the count, not a full page_size.
        assert_eq!(paginate(30, 14, 2), (2, 28, 30, 3));
    }

    #[test]
    fn an_overshot_request_clamps_to_the_last_page() {
        // A stale page index (e.g. after a filter narrowed the list) can't strand
        // the reader on a blank page.
        assert_eq!(paginate(30, 14, 99), (2, 28, 30, 3));
    }

    #[test]
    fn an_empty_list_is_one_empty_page() {
        assert_eq!(paginate(0, 14, 3), (0, 0, 0, 1));
    }

    #[test]
    fn a_zero_page_size_never_divides_by_zero() {
        assert_eq!(paginate(5, 0, 0), (0, 0, 1, 5));
    }
}
