//! The chart dashboard screen.

use crate::prelude::*;
use frus_widgets::{column, row};

/// Colours of the **extra** series (1..; series 0 takes the theme's accent).
pub(crate) const CHART_COLORS: [Color; 2] = [
    Color {
        r: 220.0 / 255.0,
        g: 120.0 / 255.0,
        b: 80.0 / 255.0,
        a: 1.0,
    },
    Color {
        r: 90.0 / 255.0,
        g: 158.0 / 255.0,
        b: 242.0 / 255.0,
        a: 1.0,
    },
];

/// Builds the dashboard's chart according to `app.chart_kind` (milestone 219): lines (0),
/// stacked areas (1), grouped bars (2), stacked bars (3). Every variant shares the same data,
/// the same axis and the same `chart_hidden` visibility state. `legend` wires up (or leaves out)
/// the clickable legend — useful for a **companion** chart that does not repeat its own.
pub(crate) fn dashboard_chart(
    app: &TodoApp,
    kind: usize,
    height: f32,
    legend: bool,
) -> Box<dyn Widget<Msg>> {
    let hidden = app.chart_hidden.clone();
    let cats = (0..5).map(|i| (CHART_CATS[i], CHART_SERIES[0].1[i]));
    if kind < 2 {
        let mut c = LineChart::new(cats)
            .height(height)
            .grid(4)
            .name(CHART_SERIES[0].0)
            .series(CHART_SERIES[1].0, CHART_COLORS[0], CHART_SERIES[1].1)
            .series(CHART_SERIES[2].0, CHART_COLORS[1], CHART_SERIES[2].1)
            .hidden(hidden)
            .animated(true);
        if kind == 1 {
            c = c.stacked(true).normalized(app.chart_normalized);
        }
        if legend {
            // The main chart: a clickable legend + clickable points (milestone 221) + the
            // selected point highlighted (milestone 223).
            c = c
                .legend(true)
                .on_legend(Msg::ChartToggleSeries)
                .on_point(Msg::ChartPoint)
                .selected(app.chart_sel);
        }
        Box::new(c)
    } else {
        let mut c = BarChart::new(cats)
            .height(height)
            .grid(4)
            .name(CHART_SERIES[0].0)
            .series(CHART_SERIES[1].0, CHART_COLORS[0], CHART_SERIES[1].1)
            .series(CHART_SERIES[2].0, CHART_COLORS[1], CHART_SERIES[2].1)
            .hidden(hidden);
        if kind == 3 {
            c = c.stacked(true).normalized(app.chart_normalized);
        }
        if legend {
            // The main chart: a clickable legend + clickable bars (milestone 222) + the selected
            // bar highlighted (milestone 223).
            c = c
                .legend(true)
                .on_legend(Msg::ChartToggleSeries)
                .on_point(Msg::ChartPoint)
                .selected(app.chart_sel);
        }
        Box::new(c)
    }
}

/// The **chart dashboard** screen: a `SegmentedButton` picks the kind (lines / stacked areas /
/// grouped bars / stacked bars, milestone 219), and the **clickable** legend hides or shows a
/// series (milestone 215/218). It demonstrates routing sub-region clicks into the state.
pub(crate) fn charts_screen(app: &TodoApp, theme: &Theme) -> Box<dyn Widget<Msg>> {
    // The window this screen fills, read from the surface description in force:
    // nothing hands it down any more.
    let Size { width, height } = surface();
    let selector = SegmentedButton::new(app.chart_kind, Msg::SetChartKind)
        .segment("Lines")
        .segment("Stacked area")
        .segment("Grouped bars")
        .segment("Stacked bars");
    // The **100%** toggle (milestone 224): only shown for the stacked kinds (stacked areas/bars),
    // where normalising means something.
    let stacked_kind = app.chart_kind == 1 || app.chart_kind == 3;
    let normalize_row: Box<dyn Widget<Msg>> = if stacked_kind {
        Box::new(
            row![
                text("100% stacking").size(13.0).color(theme.muted),
                Switch::new(app.chart_normalized).on_toggle(Msg::SetChartNormalized)
            ]
            .gap(10.0)
            .align(Align::Center),
        )
    } else {
        // Nothing to show: an empty box says that more plainly than a zero-sized
        // container with a colour it never uses.
        Box::new(SizedBox::empty())
    };
    let chart = dashboard_chart(app, app.chart_kind, 240.0, true);
    // The **companion** chart: the complementary family (bars when the main one is lines, and the
    // other way round), without a legend of its own — it shares `chart_hidden`, so hiding a series
    // through the main chart's legend hides it here **too** (milestone 220).
    let companion_kind = if app.chart_kind < 2 { 2 } else { 0 };
    let companion = dashboard_chart(app, companion_kind, 150.0, false);
    let hint = text(
        "Click a legend entry to toggle a series; click a point to pin it, or again to unpin.",
    )
    .size(13.0)
    .color(theme.muted)
    .wrap();
    // The pinned detail of the last clicked point (milestone 221).
    let pinned: Box<dyn Widget<Msg>> = match &app.chart_pin {
        Some(detail) => Box::new(Chip::new(detail.clone())),
        None => Box::new(text("No point selected").size(13.0).color(theme.muted)),
    };
    let content = column![
        row![selector].align(Align::Center),
        normalize_row,
        chart,
        row![pinned].align(Align::Center),
        text("Companion view").size(13.0).color(theme.muted),
        companion,
        hint
    ]
    .gap(16.0)
    .padding(24.0);
    // Tall fixed content (the charts + the companion, ≈ 550-650 px): it scrolls **vertically** under the bar.
    let body = SingleChildScrollView::new()
        .width(width)
        .flex(1.0)
        .child(content);
    let screen = column![NavigationBar::new("Charts").on_back(Msg::Pop), body].flex(1.0);
    Box::new(
        Container::new()
            .width(width)
            .height(height)
            .color(theme.background)
            // The background runs **under** the bars; the content does not. `SafeArea`
            // reads the intrusions from the surface description, so a screen with no
            // `Scaffold` to do it for it still keeps clear of the notch.
            .child(SafeArea::new(screen)),
    )
}
