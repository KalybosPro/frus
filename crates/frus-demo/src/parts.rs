//! The pieces more than one screen draws. Nothing arrives here on suspicion that
//! it might be shared later; it moves in when the second screen asks for it.

use crate::prelude::*;
use frus_widgets::column;

/// A statistic tile (a big number + a label) for the grid.
pub(crate) fn stat_tile(theme: &Theme, label: &str, value: usize) -> Container<Msg> {
    Container::new()
        .height(64.0)
        .radius(10.0)
        .color(theme.surface)
        .border(1.0, theme.border)
        .padding_each(10.0, 12.0, 10.0, 12.0)
        .child(column![
            text(value.to_string()).size(24.0),
            text(label.to_string()).size(13.0).color(theme.muted),
        ])
}

/// Day of the week (0 = Sunday … 6 = Saturday) of a date (Sakamoto) — milestone 238.
pub(crate) fn weekday(y: i32, m: u32, d: u32) -> u32 {
    const T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let yy = if m < 3 { y - 1 } else { y };
    (((yy + yy / 4 - yy / 100 + yy / 400 + T[(m - 1) as usize] + d as i32) % 7 + 7) % 7) as u32
}

/// True when the date falls on a **weekend** (Saturday or Sunday).
pub(crate) fn is_weekend(y: i32, m: u32, d: u32) -> bool {
    matches!(weekday(y, m, d), 0 | 6)
}

/// The showcase calendar: `DatePicker::filtered`, greying out **weekends** when `weekdays_only`
/// is set (milestone 238), otherwise every day is clickable (`DatePicker::new`).
pub(crate) fn demo_calendar(app: &TodoApp) -> Box<dyn Widget<Msg>> {
    if app.weekdays_only {
        Box::new(DatePicker::filtered(
            app.year,
            app.month,
            app.selected_day,
            |(y, m, d)| !is_weekend(y, m, d),
            Msg::PickDay,
            Msg::NavMonth,
        ))
    } else {
        Box::new(DatePicker::new(
            app.year,
            app.month,
            app.selected_day,
            Msg::PickDay,
            Msg::NavMonth,
        ))
    }
}

/// The "Stats" section: a responsive master-detail layout (`TwoPane`). Side by side when large,
/// a single pane when narrow (tapping a metric opens the detail).
pub(crate) fn stats_section(app: &TodoApp, theme: &Theme, class: SizeClass) -> TwoPane<Msg> {
    let total = app.todos.len();
    let metrics = [
        ("Total tasks", total),
        ("Active tasks", active_count(app)),
        ("Completed", done_count(app)),
    ];

    // The master pane: the list of metrics (a selection).
    let mut cats = Flex::column().gap(6.0);
    for (i, (label, _)) in metrics.iter().enumerate() {
        let variant = if app.stat_sel == i {
            Variant::Primary
        } else {
            Variant::Secondary
        };
        cats = cats.child(
            button(*label, Msg::SelectStat(i))
                .variant(variant)
                .size(15.0),
        );
    }
    let list = Card::new().padding(12.0).child(cats);

    // The detail pane: the selected metric.
    let (label, value) = metrics[app.stat_sel.min(metrics.len() - 1)];
    let mut detail_col = column![
        text(label).size(22.0),
        text(value.to_string()).size(44.0).color(theme.primary),
        text("Detail for the selected metric.")
            .size(14.0)
            .color(theme.muted),
    ]
    .gap(10.0);
    // In single-pane mode, a way back to the list.
    if class != SizeClass::Expanded {
        detail_col = detail_col.child(
            button("← Back", Msg::CloseDetail)
                .variant(Variant::Secondary)
                .size(15.0),
        );
    }
    let detail = Card::new().padding(20.0).child(detail_col);

    TwoPane::new(class)
        .ratio(0.36)
        .show_detail(app.stat_detail_open)
        .list(list)
        .detail(detail)
}

/// The "About" section: static introductory content.
pub(crate) fn about_section(theme: &Theme, width: f32) -> Container<Msg> {
    // The content width = the viewport minus the paddings (the container's 24×2 + the card's
    // 20×2), bounded to a comfortable measure — otherwise it overflows horizontally in Compact.
    let content_width = (width - 88.0).clamp(240.0, 560.0);
    Container::new().padding(24.0).child(
        Card::new().padding(20.0).child(
            column![
                text("About frus").size(24.0),
                // Rich text: mixed styles on one line, with cascading inheritance.
                RichText::new(
                    TextSpan::new("A ")
                        .child(TextSpan::new("fast").bold())
                        .child(TextSpan::new(", "))
                        .child(TextSpan::new("portable").italic().underline())
                        .child(TextSpan::new(" Rust UI framework — "))
                        .child(TextSpan::new("no GC").bold().color(theme.primary))
                        .child(TextSpan::new(".")),
                )
                .base_style(theme.text.body_medium.color(theme.muted))
                .wrap(),
                Divider::new(),
                Timeline::new()
                    .event("Responsive primitives", "Milestone 42")
                    .event("Adaptive navigation", "Milestone 43"),
                // A paragraph: it wraps at the card's width.
                text(
                    "Layout, painting, typography and animation are engine-level \
                     foundations shared by every widget in this gallery.",
                )
                .size(13.0)
                .color(theme.muted)
                .wrap(),
            ]
            .gap(12.0)
            .width(content_width),
        ),
    )
}
