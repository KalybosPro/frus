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

/// The **chart dashboard** screen: a `SegmentedButton` picks the kind (lines / stacked areas /
/// grouped bars / stacked bars, milestone 219), and the **clickable** legend hides or shows a
/// series (milestone 215/218). It demonstrates routing sub-region clicks into the state.
pub(crate) struct ChartsPage;

/// What the dashboard keeps.
#[derive(Default)]
pub(crate) struct ChartsState {
    /// Indices of the chart's **hidden** series (toggled through the legend, milestone 218).
    pub(crate) hidden: Vec<usize>,
    /// The kind of chart shown: 0 lines, 1 stacked areas, 2 grouped bars, 3 stacked bars
    /// (milestone 219).
    pub(crate) kind: usize,
    /// The **pinned** detail of a clicked point (`series · category = value`), if there is one
    /// (milestone 221).
    pub(crate) pin: Option<String>,
    /// The **selected** point/bar `(category, series)`, highlighted in the chart (milestone 223).
    pub(crate) selected: Option<(usize, usize)>,
    /// **100%** stacking (proportions) turned on for the stacked charts (milestone 224).
    pub(crate) normalized: bool,
}

impl ChartsState {
    /// A legend click: hides the series when it is visible, shows it again otherwise.
    pub(crate) fn toggle_series(&mut self, index: usize) {
        if let Some(pos) = self.hidden.iter().position(|&h| h == index) {
            self.hidden.remove(pos);
        } else {
            self.hidden.push(index);
        }
    }

    /// Clicking the already-selected item again **unpins** it (milestone 225). Otherwise it pins
    /// "series · category = value" and remembers the selection `(cat, series)` (milestones
    /// 221/223).
    pub(crate) fn click_point(&mut self, cat: usize, series: usize) {
        if self.selected == Some((cat, series)) {
            self.selected = None;
            self.pin = None;
        } else if let (Some((name, values)), Some(label)) =
            (CHART_SERIES.get(series), CHART_CATS.get(cat))
        {
            if let Some(v) = values.get(cat) {
                self.pin = Some(format!("{name} · {label} = {}", *v as i64));
                self.selected = Some((cat, series));
            }
        }
    }
}

impl StatefulWidget for ChartsPage {
    type State = ChartsState;

    fn create_state(&self) -> ChartsState {
        ChartsState::default()
    }
}

impl State for ChartsState {
    type Widget = ChartsPage;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let theme = cx.theme().clone();
        let theme = &theme;
        // The window this screen fills, read from the surface description in force:
        // nothing hands it down any more.
        let Size { width, height } = surface();
        let selector = SegmentedButton::new(self.kind, cx.handler(|s, k: usize| s.kind = k))
            .segment("Lines")
            .segment("Stacked area")
            .segment("Grouped bars")
            .segment("Stacked bars");
        // The **100%** toggle (milestone 224): only shown for the stacked kinds (stacked
        // areas/bars), where normalising means something.
        let stacked_kind = self.kind == 1 || self.kind == 3;
        let normalize_row: Box<dyn Widget> = if stacked_kind {
            Box::new(
                row![
                    text("100% stacking").size(13.0).color(theme.muted),
                    Switch::new(self.normalized)
                        .on_toggle(cx.handler(|s, on: bool| s.normalized = on))
                ]
                .gap(10.0)
                .align(Align::Center),
            )
        } else {
            // Nothing to show: an empty box says that more plainly than a zero-sized
            // container with a colour it never uses.
            Box::new(SizedBox::empty())
        };
        let chart = self.dashboard_chart(cx, self.kind, 240.0, true);
        // The **companion** chart: the complementary family (bars when the main one is lines,
        // and the other way round), without a legend of its own — it shares `hidden`, so hiding
        // a series through the main chart's legend hides it here **too** (milestone 220).
        let companion_kind = if self.kind < 2 { 2 } else { 0 };
        let companion = self.dashboard_chart(cx, companion_kind, 150.0, false);
        let hint = text(
            "Click a legend entry to toggle a series; click a point to pin it, or again to unpin.",
        )
        .size(13.0)
        .color(theme.muted)
        .wrap();
        // The pinned detail of the last clicked point (milestone 221).
        let pinned: Box<dyn Widget> = match &self.pin {
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
        // Tall fixed content (the charts + the companion, ≈ 550-650 px): it scrolls
        // **vertically** under the bar.
        let body = SingleChildScrollView::new()
            .width(width)
            .flex(1.0)
            .child(content);
        let router = cx.router();
        let screen = column![
            NavigationBar::new("Charts").on_back(on(move || {
                router.pop();
            })),
            body
        ]
        .flex(1.0);
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
}

impl ChartsState {
    /// Builds the dashboard's chart according to `kind` (milestone 219): lines (0), stacked
    /// areas (1), grouped bars (2), stacked bars (3). Every variant shares the same data, the
    /// same axis and the same `hidden` visibility state. `legend` wires up (or leaves out) the
    /// clickable legend — useful for a **companion** chart that does not repeat its own.
    pub(crate) fn dashboard_chart(
        &self,
        cx: &StateContext<Self>,
        kind: usize,
        height: f32,
        legend: bool,
    ) -> Box<dyn Widget> {
        let hidden = self.hidden.clone();
        let cats = (0..5).map(|i| (CHART_CATS[i], CHART_SERIES[0].1[i]));
        let toggle = cx.handler(|s, series: usize| s.toggle_series(series));
        let point = {
            let handle = cx.handle();
            move |cat: usize, series: usize| {
                let handle = handle.clone();
                Callback::new(move || handle.set_state(|s| s.click_point(cat, series)))
            }
        };
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
                c = c.stacked(true).normalized(self.normalized);
            }
            if legend {
                // The main chart: a clickable legend + clickable points (milestone 221) + the
                // selected point highlighted (milestone 223).
                c = c
                    .legend(true)
                    .on_legend(toggle)
                    .on_point(point)
                    .selected(self.selected);
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
                c = c.stacked(true).normalized(self.normalized);
            }
            if legend {
                // The main chart: a clickable legend + clickable bars (milestone 222) + the
                // selected bar highlighted (milestone 223).
                c = c
                    .legend(true)
                    .on_legend(toggle)
                    .on_point(point)
                    .selected(self.selected);
            }
            Box::new(c)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_legend_toggle_hides_and_shows_series() {
        let mut s = ChartsState::default();
        s.toggle_series(1);
        assert_eq!(s.hidden, [1]);
        s.toggle_series(2);
        assert_eq!(s.hidden, [1, 2]);
        s.toggle_series(1);
        assert_eq!(s.hidden, [2], "a second click shows it again");
    }

    #[test]
    fn clicking_a_point_pins_its_detail_and_again_unpins_it() {
        let mut s = ChartsState::default();
        s.click_point(1, 0);
        assert_eq!(s.pin.as_deref(), Some("Sales · Tue = 7"));
        assert_eq!(s.selected, Some((1, 0)));
        s.click_point(2, 1);
        assert_eq!(s.pin.as_deref(), Some("Costs · Wed = 3"));
        s.click_point(2, 1);
        assert_eq!((s.pin.as_deref(), s.selected), (None, None));
        s.click_point(9, 9);
        assert_eq!(s.selected, None, "a point that is not there pins nothing");
    }
}
