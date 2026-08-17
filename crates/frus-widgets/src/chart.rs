//! The "charts" domain: self-painted, themed **data views**.
//!
//! - [`BarChart`]: a series of `(label, value)` as vertical bars scaled to the largest value,
//!   the value above, the label below, a baseline.
//! - [`LineChart`]: the same series drawn as a **polyline** (segments joining the points, round
//!   markers), to read a trend rather than compare magnitudes.
//!
//! Both are purely **self-painted** (no children) and not generic over `Msg` (like
//! [`crate::Icon`]): they are data views, not controls.

use frus_core::{Color, Path, Point, Rect, Scene};
use frus_layout::{Dimension, Style};

use crate::interaction::Status;
use crate::theme::Theme;
use crate::widget::Widget;

/// Default height of the chart (logical px).
const DEFAULT_HEIGHT: f32 = 200.0;
/// Band reserved for the category labels below the baseline.
const X_LABEL_H: f32 = 22.0;
/// Font size of the values (above the bars) and of the labels (below).
const VALUE_SIZE: f32 = 12.0;
const LABEL_SIZE: f32 = 12.0;
/// Font size of the share (`%`) written inside a stratum in 100% mode (milestone 227).
const STRATA_LABEL_SIZE: f32 = 11.0;
/// Fraction of a bar's "slot" actually taken by the bar (the rest = the spacing).
const BAR_FILL: f32 = 0.6;
/// Width of the left margin reserved for the y-axis ticks (when there is an axis).
const Y_AXIS_W: f32 = 34.0;
/// Font size of the y-axis ticks.
const AXIS_SIZE: f32 = 11.0;

/// A bar chart.
///
/// ```
/// use frus_widgets::BarChart;
/// let chart: BarChart = BarChart::new([("Mon", 3.0), ("Tue", 5.0), ("Wed", 2.0)]).height(160.0);
/// ```
pub struct BarChart<Msg = ()> {
    values: Vec<(String, f32)>,
    /// Colour of the bars; `None` = the theme's `primary`.
    color: Option<Color>,
    height: f32,
    /// Number of y-axis divisions (grid lines + ticks); `0` = no axis.
    grid: usize,
    /// Name of the main series (for the legend); `None` = anonymous.
    name: Option<String>,
    /// **Extra** series `(name, colour, values)` — bars **grouped** by category.
    extra: Vec<(String, Color, Vec<f32>)>,
    /// Show a legend (one swatch + name per series)?
    legend: bool,
    /// Indices of **hidden** series (not drawn, dimmed in the legend) — milestone 215.
    hidden: Vec<usize>,
    /// Message emitted on a click on a legend entry (the series index) — milestone 215.
    on_legend: Option<Box<dyn Fn(usize) -> Msg>>,
    /// Stack the series (cumulative bars) rather than group them? — milestone 216.
    stacked: bool,
    /// Message emitted on a click on a **bar** `(category, series)` — milestone 222.
    on_point: Option<Box<dyn Fn(usize, usize) -> Msg>>,
    /// **Pinned** bar/stratum `(category, series)`, highlighted by a persistent ring —
    /// milestone 223.
    selected: Option<(usize, usize)>,
    /// **100%** stacking: each column is normalised to its own total (proportions) — milestone 224.
    normalized: bool,
}

impl<Msg> BarChart<Msg> {
    /// Creates a chart from a series of `(label, value)`.
    pub fn new(data: impl IntoIterator<Item = (impl Into<String>, f32)>) -> Self {
        Self {
            values: data
                .into_iter()
                .map(|(l, v)| (l.into(), v.max(0.0)))
                .collect(),
            color: None,
            height: DEFAULT_HEIGHT,
            grid: 0,
            name: None,
            extra: Vec::new(),
            legend: false,
            hidden: Vec::new(),
            on_legend: None,
            stacked: false,
            on_point: None,
            selected: None,
            normalized: false,
        }
    }

    /// Overrides the colour of the bars (default: the theme's `primary`).
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Height of the chart in logical pixels (200 by default).
    pub fn height(mut self, height: f32) -> Self {
        self.height = height.max(X_LABEL_H + VALUE_SIZE + 8.0);
        self
    }

    /// Adds a **y-axis**: `divisions` horizontal grid lines with their ticks (`0..max`) in a
    /// left margin. `0` (the default) = no axis.
    pub fn grid(mut self, divisions: usize) -> Self {
        self.grid = divisions;
        self
    }

    /// Names the **main** series (displayed in the legend).
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Adds an **extra series** `(name, colour, values)`, drawn as bars **grouped** side by
    /// side within each category. Every series shares the scale and the axis.
    pub fn series(
        mut self,
        name: impl Into<String>,
        color: Color,
        values: impl IntoIterator<Item = f32>,
    ) -> Self {
        self.extra.push((
            name.into(),
            color,
            values.into_iter().map(|v| v.max(0.0)).collect(),
        ));
        self
    }

    /// Shows a **legend** (colour swatch + name) for each series. Off by default.
    pub fn legend(mut self, legend: bool) -> Self {
        self.legend = legend;
        self
    }

    /// **Hides** the series at the given indices (not drawn, dimmed in the legend) — milestone 215.
    pub fn hidden(mut self, indices: impl IntoIterator<Item = usize>) -> Self {
        self.hidden = indices.into_iter().collect();
        self
    }

    /// Makes the **legend clickable**: `on_legend(index)` on a click on an entry — milestone 215.
    pub fn on_legend(mut self, on_legend: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_legend = Some(Box::new(on_legend));
        self
    }

    /// **Stacks** the series: one bar per category, segmented by series (cumulative bars),
    /// instead of bars grouped side by side (milestone 212). Off by default.
    pub fn stacked(mut self, stacked: bool) -> Self {
        self.stacked = stacked;
        self
    }

    /// Makes the **bars clickable**: `on_point(category, series)` on a click on a visible bar
    /// (or stacked stratum). None by default — milestone 222.
    pub fn on_point(mut self, on_point: impl Fn(usize, usize) -> Msg + 'static) -> Self {
        self.on_point = Some(Box::new(on_point));
        self
    }

    /// **Pins** a bar/stratum `(category, series)`: it gets a persistent accent ring
    /// (highlighting the current selection, as a clicked detail). `None` = nothing — milestone 223.
    pub fn selected(mut self, selected: Option<(usize, usize)>) -> Self {
        self.selected = selected;
        self
    }

    /// Normalises the stacking to **100%**: each column fills the whole height, each stratum
    /// taking its **share** of the category's total (rather than its absolute value). Only has
    /// an effect in multi-series stacked mode. Off by default — milestone 224.
    pub fn normalized(mut self, normalized: bool) -> Self {
        self.normalized = normalized;
        self
    }

    /// The largest value across **all** the series (at least 1, for a stable scale).
    fn max_value(&self) -> f32 {
        let primary = self.values.iter().map(|(_, v)| *v);
        let extra = self.extra.iter().flat_map(|(_, _, vs)| vs.iter().copied());
        primary.chain(extra).fold(0.0, f32::max).max(1.0)
    }

    /// Total (of the **visible** series) of category `i` — the denominator of 100% stacking.
    fn category_total(&self, i: usize) -> f32 {
        let base = if self.hidden.contains(&0) {
            0.0
        } else {
            self.values.get(i).map(|(_, v)| *v).unwrap_or(0.0)
        };
        let rest: f32 = self
            .extra
            .iter()
            .enumerate()
            .filter(|(j, _)| !self.hidden.contains(&(j + 1)))
            .map(|(_, (_, _, vs))| vs.get(i).copied().unwrap_or(0.0))
            .sum();
        (base + rest).max(1e-6)
    }

    /// The largest **sum** of the series per category (the scale in stacked mode).
    fn stacked_max(&self) -> f32 {
        let n = self.values.len();
        (0..n)
            .map(|i| {
                self.values[i].1
                    + self
                        .extra
                        .iter()
                        .map(|(_, _, vs)| vs.get(i).copied().unwrap_or(0.0))
                        .sum::<f32>()
            })
            .fold(0.0, f32::max)
            .max(1.0)
    }

    /// Should the legend be drawn (enabled **and** at least one named series)?
    fn has_legend(&self) -> bool {
        self.legend && (self.name.is_some() || !self.extra.is_empty())
    }

    /// Names of every series (the main one then the extras) — for the legend / for routing.
    fn series_names(&self) -> Vec<&str> {
        let mut names = vec![self.name.as_deref().unwrap_or("Series 1")];
        names.extend(self.extra.iter().map(|(n, _, _)| n.as_str()));
        names
    }
}

/// Formats a value: as an integer if it is one, otherwise with one decimal.
fn format_value(v: f32) -> String {
    if (v.fract()).abs() < 1e-6 {
        format!("{}", v as i64)
    } else {
        format!("{v:.1}")
    }
}

/// Formats a measure for the tooltip: the raw value, followed by its **share** (`%`) of the total
/// when a 100%-stacking denominator is supplied (milestone 226). `None` = the value alone.
fn format_measure(value: f32, percent_of: Option<f32>) -> String {
    match percent_of {
        Some(total) if total > 0.0 => {
            format!(
                "{} ({}%)",
                format_value(value),
                (value / total * 100.0).round() as i64
            )
        }
        _ => format_value(value),
    }
}

/// Width of the axis margin if `divisions > 0`, otherwise `0` (shared by BarChart / LineChart).
fn axis_width(divisions: usize) -> f32 {
    if divisions > 0 {
        Y_AXIS_W
    } else {
        0.0
    }
}

/// Draws the **y-axis**: `divisions` horizontal grid lines from `plot_left` to
/// `plot_left + plot_w`, spread between the baseline and the top of the plot area, each labelled
/// with its value (`0..max`), right-aligned in the left margin. Shared by both charts — the grid
/// reads behind the bars or the curve.
#[allow(clippy::too_many_arguments)]
fn draw_grid(
    scene: &mut Scene,
    theme: &Theme,
    plot_left: f32,
    plot_w: f32,
    plot_top: f32,
    baseline_y: f32,
    max: f32,
    divisions: usize,
    percent: bool,
    opacity: f32,
) {
    if divisions == 0 {
        return;
    }
    let plot_h = baseline_y - plot_top;
    for i in 0..=divisions {
        let t = i as f32 / divisions as f32;
        let y = baseline_y - plot_h * t;
        // A grid line (except i == 0: that is the baseline, already drawn by the chart).
        if i > 0 {
            scene.fill_rect(
                Rect::new(plot_left, y, plot_w, 1.0),
                theme.scheme.outline_variant.fade(opacity * 0.6),
            );
        }
        // A tick: the value (or the percentage in 100% mode), right-aligned in the margin.
        let label = if percent {
            format!("{}%", (t * 100.0).round() as i64)
        } else {
            format_value(max * t)
        };
        let lw = frus_text::measure(&label, AXIS_SIZE).width;
        scene.text(
            Point::new(plot_left - 6.0 - lw, y - AXIS_SIZE * 0.5),
            label,
            AXIS_SIZE,
            theme.muted.fade(opacity),
        );
    }
}

/// Draws a **legend** (colour swatch + name, from left to right) in the top band.
/// Shared by BarChart / LineChart (milestone 209/212).
fn draw_legend(
    scene: &mut Scene,
    theme: &Theme,
    left: f32,
    top: f32,
    series: &[(Color, &str)],
    o: f32,
) {
    let mut x = left;
    let sy = top + (LEGEND_H - LEGEND_SWATCH) * 0.5;
    for (color, name) in series {
        scene.draw_rect(
            Rect::new(x, sy, LEGEND_SWATCH, LEGEND_SWATCH),
            color.fade(o),
            2.0,
            0.0,
            Color::TRANSPARENT,
        );
        x += LEGEND_SWATCH + 5.0;
        scene.text(
            Point::new(x, top + (LEGEND_H - LEGEND_SIZE) * 0.5),
            (*name).to_string(),
            LEGEND_SIZE,
            theme.muted.fade(o),
        );
        x += frus_text::measure(name, LEGEND_SIZE).width + 16.0;
    }
}

/// Which **legend entry** contains the local point `(x, y)`? Rebuilds [`draw_legend`]'s layout to
/// route a click to the series index. Shared (milestone 215).
fn legend_hit(local_x: f32, local_y: f32, plot_left: f32, names: &[&str]) -> Option<usize> {
    if !(0.0..=LEGEND_H).contains(&local_y) {
        return None;
    }
    let mut x = plot_left;
    for (i, name) in names.iter().enumerate() {
        let entry_w = LEGEND_SWATCH + 5.0 + frus_text::measure(name, LEGEND_SIZE).width;
        if local_x >= x && local_x <= x + entry_w {
            return Some(i);
        }
        x += entry_w + 16.0;
    }
    None
}

/// Draws a hover **tooltip**: a vertical guide at `gx`, then a box listing `lines` (each line: an
/// optional swatch + text). The box is sized to the longest label, placed to the right of the guide
/// (flipped to the left if it overflows), anchored at the top of the plot area.
/// Shared by BarChart / LineChart (milestone 211/212).
#[allow(clippy::too_many_arguments)]
fn draw_tooltip(
    scene: &mut Scene,
    theme: &Theme,
    bounds: Rect,
    gx: f32,
    plot_top: f32,
    baseline_y: f32,
    lines: &[(Option<Color>, String)],
    o: f32,
) {
    scene.fill_rect(
        Rect::new(gx - 0.5, plot_top, 1.0, baseline_y - plot_top),
        theme.scheme.outline_variant.fade(o * 0.8),
    );
    let pad = 8.0;
    let line_h = TOOLTIP_SIZE + 5.0;
    let dot = 7.0;
    let text_w = lines
        .iter()
        .map(|(c, t)| {
            let lead = if c.is_some() { dot + 5.0 } else { 0.0 };
            lead + frus_text::measure(t, TOOLTIP_SIZE).width
        })
        .fold(0.0, f32::max);
    let bw = text_w + pad * 2.0;
    let bh = lines.len() as f32 * line_h + pad * 2.0 - 3.0;
    let mut bx = gx + 12.0;
    if bx + bw > bounds.x + bounds.width {
        bx = gx - 12.0 - bw;
    }
    bx = bx.max(bounds.x);
    let by = plot_top.max(bounds.y);
    scene.draw_rect(
        Rect::new(bx, by, bw, bh),
        theme.surface.fade(o),
        6.0,
        1.0,
        theme.scheme.outline_variant.fade(o),
    );
    let mut ty = by + pad;
    for (c, t) in lines {
        let mut tx = bx + pad;
        if let Some(col) = c {
            scene.fill_path(
                &Path::circle(
                    Point::new(tx + dot * 0.5, ty + TOOLTIP_SIZE * 0.5),
                    dot * 0.5,
                ),
                col.fade(o),
            );
            tx += dot + 5.0;
        }
        scene.text(
            Point::new(tx, ty),
            t.clone(),
            TOOLTIP_SIZE,
            theme.on_surface.fade(o),
        );
        ty += line_h;
    }
}

/// Is the local point `(x, y)` inside the **plot area**? If so, returns `Cursor::Default` (which
/// turns on pointer tracking for the tooltip without changing the cursor's shape — a chart is not
/// clickable), otherwise `None`. Shared by BarChart / LineChart (milestone 211/212).
fn chart_plot_hit(
    local_x: f32,
    local_y: f32,
    width: f32,
    height: f32,
    plot_left: f32,
    plot_top: f32,
) -> Option<crate::interaction::Cursor> {
    let baseline_y = height - X_LABEL_H;
    if local_x >= plot_left && local_x <= width && local_y >= plot_top && local_y <= baseline_y {
        Some(crate::interaction::Cursor::Default)
    } else {
        None
    }
}

impl<Msg> Widget<Msg> for BarChart<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Percent(1.0),
            height: Dimension::Length(self.height),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let n = self.values.len();
        if n == 0 {
            return;
        }
        let o = status.opacity;
        let accent = self.color.unwrap_or(theme.primary);
        let single = self.extra.is_empty();
        let stacked = self.stacked && !single;
        // **100%** stacking (milestone 224): each column is normalised to its own total (proportions).
        let normalized = self.normalized && stacked;
        // When stacked, the scale must hold the cumulative **total** per category.
        let max = if stacked {
            self.stacked_max()
        } else {
            self.max_value()
        };

        // Plot area: below the band of values, above the category labels; a left margin for the
        // axis, and — where there is one — a legend band right at the top.
        let legend_h = if self.has_legend() { LEGEND_H } else { 0.0 };
        let baseline_y = bounds.y + bounds.height - X_LABEL_H;
        let plot_top = bounds.y + legend_h + VALUE_SIZE + 6.0;
        let plot_h = (baseline_y - plot_top).max(1.0);
        let axis_w = axis_width(self.grid);
        let plot_left = bounds.x + axis_w;
        let plot_w = bounds.width - axis_w;
        let slot = plot_w / n as f32;

        // Horizontal grid + ticks (behind the bars); in 100% mode the axis is in percentages.
        draw_grid(
            scene, theme, plot_left, plot_w, plot_top, baseline_y, max, self.grid, normalized, o,
        );
        // Baseline (the x-axis).
        scene.fill_rect(
            Rect::new(plot_left, baseline_y, plot_w, 1.5),
            theme.scheme.outline_variant.fade(o),
        );

        // Every series: the main one then the extras (grouped bars).
        let primary: Vec<f32> = self.values.iter().map(|(_, v)| *v).collect();
        let mut series: Vec<(Color, &str, &[f32])> = vec![(
            accent,
            self.name.as_deref().unwrap_or("Series 1"),
            primary.as_slice(),
        )];
        for (name, color, vals) in &self.extra {
            series.push((*color, name.as_str(), vals.as_slice()));
        }
        let s = series.len();

        // Each category: either a group of `s` bars side by side (milestone 212), or — when
        // stacked — a single bar segmented by series (cumulative bars, milestone 216).
        let group_w = slot * BAR_FILL;
        let bar_w = group_w / s as f32;
        let inner = if s == 1 { 1.0 } else { 0.86 };
        // Rect of the pinned bar/stratum, captured to draw its ring afterwards (milestone 223).
        let mut sel_rect: Option<Rect> = None;
        for i in 0..n {
            let cx = plot_left + slot * (i as f32 + 0.5);
            if stacked {
                // Segments stacked from the bottom up (hidden series do not count). In 100% mode
                // the denominator is the category's total (a full column), otherwise the scale.
                let sbx = cx - group_w * 0.5;
                let denom = if normalized {
                    self.category_total(i)
                } else {
                    max
                };
                let mut lower = 0.0_f32;
                for (j, (color, _, vals)) in series.iter().enumerate() {
                    if self.hidden.contains(&j) {
                        continue;
                    }
                    let value = vals.get(i).copied().unwrap_or(0.0);
                    let y_bottom = baseline_y - (lower / denom) * plot_h;
                    let y_top = baseline_y - ((lower + value) / denom) * plot_h;
                    let rect = Rect::new(sbx, y_top, group_w, y_bottom - y_top);
                    scene.draw_rect(rect, color.fade(o), 0.0, 0.0, Color::TRANSPARENT);
                    if self.selected == Some((i, j)) {
                        sel_rect = Some(rect);
                    }
                    // Label centred in the stratum if it is tall enough to hold it: the share (%)
                    // in 100% mode (milestone 227), the raw value in absolute stacked mode
                    // (milestone 229). Text legible over a saturated background.
                    let seg_h = y_bottom - y_top;
                    if seg_h >= STRATA_LABEL_SIZE + 4.0 {
                        let label = if normalized {
                            format!("{}%", (value / denom * 100.0).round() as i64)
                        } else {
                            format_value(value)
                        };
                        let lw = frus_text::measure(&label, STRATA_LABEL_SIZE).width;
                        scene.text(
                            Point::new(
                                cx - lw * 0.5,
                                (y_top + y_bottom) * 0.5 - STRATA_LABEL_SIZE * 0.5,
                            ),
                            label,
                            STRATA_LABEL_SIZE,
                            theme.on_primary.fade(o * 0.95),
                        );
                    }
                    lower += value;
                }
                // The column's total above the topmost stratum (**absolute** stacking: parity with
                // the value on plain bars; in 100% mode the column is full) — milestone 228.
                if !normalized && lower > 0.0 {
                    let vs = format_value(lower);
                    let vw = frus_text::measure(&vs, VALUE_SIZE).width;
                    let top_y = baseline_y - (lower / denom) * plot_h;
                    scene.text(
                        Point::new(cx - vw * 0.5, top_y - VALUE_SIZE - 2.0),
                        vs,
                        VALUE_SIZE,
                        theme.on_surface.fade(o),
                    );
                }
            } else {
                let group_left = cx - group_w * 0.5;
                for (j, (color, _, vals)) in series.iter().enumerate() {
                    if self.hidden.contains(&j) {
                        continue;
                    }
                    let value = vals.get(i).copied().unwrap_or(0.0);
                    let h = (value / max) * plot_h;
                    let draw_w = bar_w * inner;
                    let bx = group_left + j as f32 * bar_w + (bar_w - draw_w) * 0.5;
                    let rect = Rect::new(bx, baseline_y - h, draw_w, h);
                    scene.draw_rect(rect, color.fade(o), 4.0, 0.0, Color::TRANSPARENT);
                    if self.selected == Some((i, j)) {
                        sel_rect = Some(rect);
                    }
                    // The value above (single series only, to avoid the clutter).
                    if single {
                        let vs = format_value(value);
                        let vw = frus_text::measure(&vs, VALUE_SIZE).width;
                        scene.text(
                            Point::new(cx - vw * 0.5, baseline_y - h - VALUE_SIZE - 2.0),
                            vs,
                            VALUE_SIZE,
                            theme.on_surface.fade(o),
                        );
                    }
                }
            }
            // Category label below the baseline.
            let label = &self.values[i].0;
            let lw = frus_text::measure(label, LABEL_SIZE).width;
            scene.text(
                Point::new(cx - lw * 0.5, baseline_y + 4.0),
                label.clone(),
                LABEL_SIZE,
                theme.muted.fade(o),
            );
        }

        // Pinned bar/stratum (milestone 223): a persistent accent ring, slightly inflated around
        // the rect, in a contrasting colour (independent of hover).
        if let Some(r) = sel_rect {
            scene.draw_rect(
                Rect::new(r.x - 2.5, r.y - 2.5, r.width + 5.0, r.height + 5.0),
                Color::TRANSPARENT,
                5.0,
                2.0,
                theme.on_surface.fade(o),
            );
        }

        // Legend (the top band), shared; hidden series are dimmed in it.
        if self.has_legend() {
            let entries: Vec<(Color, &str)> = series
                .iter()
                .enumerate()
                .map(|(i, (c, name, _))| {
                    (
                        if self.hidden.contains(&i) {
                            c.fade(0.35)
                        } else {
                            *c
                        },
                        *name,
                    )
                })
                .collect();
            draw_legend(scene, theme, plot_left, bounds.y, &entries, o);
        }

        // Hover tooltip (milestone 212): the nearest category + the value of each visible series.
        if let Some(hc) = status.hover_cursor {
            let lx = hc.x - bounds.x;
            let hi =
                (((lx - plot_left) / slot - 0.5).round() as i64).clamp(0, n as i64 - 1) as usize;
            let gx = plot_left + slot * (hi as f32 + 0.5);
            let mut lines: Vec<(Option<Color>, String)> = vec![(None, self.values[hi].0.clone())];
            // In 100% mode each measure is followed by its share of the hovered category's total (milestone 226).
            let percent_of = if normalized {
                Some(self.category_total(hi))
            } else {
                None
            };
            for (j, (color, name, vals)) in series.iter().enumerate() {
                if self.hidden.contains(&j) {
                    continue;
                }
                let value = vals.get(hi).copied().unwrap_or(0.0);
                let vtxt = format_measure(value, percent_of);
                let txt = if single {
                    vtxt
                } else {
                    format!("{}  {}", name, vtxt)
                };
                lines.push((Some(*color), txt));
            }
            draw_tooltip(scene, theme, bounds, gx, plot_top, baseline_y, &lines, o);
        }
    }

    fn cursor_icon(
        &self,
        local_x: f32,
        local_y: f32,
        width: f32,
        height: f32,
    ) -> Option<crate::interaction::Cursor> {
        if self.values.is_empty() {
            return None;
        }
        let legend_h = if self.has_legend() { LEGEND_H } else { 0.0 };
        let plot_top = legend_h + VALUE_SIZE + 6.0;
        chart_plot_hit(
            local_x,
            local_y,
            width,
            height,
            axis_width(self.grid),
            plot_top,
        )
    }

    fn positional_click(&self, local_x: f32, local_y: f32, width: f32, height: f32) -> Option<Msg> {
        // 1) A click on a legend entry → on_legend(index) (milestone 215).
        if let Some(f) = &self.on_legend {
            if self.has_legend() {
                if let Some(idx) = legend_hit(
                    local_x,
                    local_y,
                    axis_width(self.grid),
                    &self.series_names(),
                ) {
                    return Some(f(idx));
                }
            }
        }
        // 2) A click on a **bar** (or a stacked stratum) → on_point(category, series) (milestone 222).
        // The geometry matches the paint: each rect is rebuilt and tested for containment.
        let f = self.on_point.as_ref()?;
        let n = self.values.len();
        if n == 0 {
            return None;
        }
        let single = self.extra.is_empty();
        let stacked = self.stacked && !single;
        let max = if stacked {
            self.stacked_max()
        } else {
            self.max_value()
        };
        let legend_h = if self.has_legend() { LEGEND_H } else { 0.0 };
        let baseline_y = height - X_LABEL_H;
        let plot_top = legend_h + VALUE_SIZE + 6.0;
        let plot_h = (baseline_y - plot_top).max(1.0);
        let plot_left = axis_width(self.grid);
        let slot = (width - plot_left) / n as f32;
        let s = 1 + self.extra.len();
        let group_w = slot * BAR_FILL;
        let bar_w = group_w / s as f32;
        let inner = if s == 1 { 1.0 } else { 0.86 };
        let primary: Vec<f32> = self.values.iter().map(|(_, v)| *v).collect();
        let value_at = |j: usize, i: usize| -> f32 {
            if j == 0 {
                primary.get(i).copied().unwrap_or(0.0)
            } else {
                self.extra[j - 1].2.get(i).copied().unwrap_or(0.0)
            }
        };
        for i in 0..n {
            let cx = plot_left + slot * (i as f32 + 0.5);
            if stacked {
                let sbx = cx - group_w * 0.5;
                let mut lower = 0.0_f32;
                for j in 0..s {
                    if self.hidden.contains(&j) {
                        continue;
                    }
                    let value = value_at(j, i);
                    let y_bottom = baseline_y - (lower / max) * plot_h;
                    let y_top = baseline_y - ((lower + value) / max) * plot_h;
                    if local_x >= sbx
                        && local_x <= sbx + group_w
                        && local_y >= y_top
                        && local_y <= y_bottom
                    {
                        return Some(f(i, j));
                    }
                    lower += value;
                }
            } else {
                let group_left = cx - group_w * 0.5;
                for j in 0..s {
                    if self.hidden.contains(&j) {
                        continue;
                    }
                    let h = (value_at(j, i) / max) * plot_h;
                    let draw_w = bar_w * inner;
                    let bx = group_left + j as f32 * bar_w + (bar_w - draw_w) * 0.5;
                    if local_x >= bx
                        && local_x <= bx + draw_w
                        && local_y >= baseline_y - h
                        && local_y <= baseline_y
                    {
                        return Some(f(i, j));
                    }
                }
            }
        }
        None
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

/// Radius (px) of the round markers placed on each point of a [`LineChart`].
const MARKER_R: f32 = 3.5;
/// Tolerance radius (px) for clicking a point (milestone 221).
const POINT_HIT_R: f32 = 12.0;
/// (Relative) opacity of the area filled under the curve.
const AREA_ALPHA: f32 = 0.16;
/// (Relative) opacity of the bands of a **stacked area** chart (stronger, so each stratum can
/// be read).
const STACK_ALPHA: f32 = 0.55;
/// Height of the legend band (above the plot area) when it is shown.
const LEGEND_H: f32 = 20.0;
/// Side of the colour swatch of a legend entry.
const LEGEND_SWATCH: f32 = 10.0;
/// Font size of the legend entries.
const LEGEND_SIZE: f32 = 12.0;
/// Font size of a tooltip's content.
const TOOLTIP_SIZE: f32 = 12.0;
/// Speed (cycles per second) of the animated pulsing halo (milestone 217).
const PULSE_SPEED: f32 = 1.6;
/// Growth (px) of the pulsing halo's radius over one cycle.
const PULSE_GROW: f32 = 10.0;
/// Thickness (px) of the polyline's stroke.
const LINE_W: f32 = 2.0;

/// A **line** chart: the same `(label, value)` series as a [`BarChart`], but joined into a
/// polyline (segments + markers) so that a **trend** can be read.
///
/// ```
/// use frus_widgets::LineChart;
/// let chart: LineChart = LineChart::new([("Mon", 3.0), ("Tue", 5.0), ("Wed", 2.0)]).height(160.0);
/// ```
pub struct LineChart<Msg = ()> {
    values: Vec<(String, f32)>,
    /// Colour of the stroke and of the markers; `None` = the theme's `primary`.
    color: Option<Color>,
    height: f32,
    /// Number of y-axis divisions (grid lines + ticks); `0` = no axis.
    grid: usize,
    /// Fill the area under the curve (a flat wash, the stroke's colour dimmed)?
    fill: bool,
    /// Name of the main series (for the legend); `None` = anonymous.
    name: Option<String>,
    /// **Extra** series `(name, colour, values)`, aligned by index onto the main series'
    /// categories.
    extra: Vec<(String, Color, Vec<f32>)>,
    /// Show a legend (one swatch + name per series)?
    legend: bool,
    /// Stack the series (cumulative areas) rather than overlay them?
    stacked: bool,
    /// Indices of **hidden** series (not drawn, dimmed in the legend) — milestone 215.
    hidden: Vec<usize>,
    /// Message emitted on a click on a legend entry (the series index) — milestone 215.
    on_legend: Option<Box<dyn Fn(usize) -> Msg>>,
    /// Animate a **pulsing halo** on the hovered point (continuous repaint) — milestone 217.
    animated: bool,
    /// Message emitted on a click on a **point** `(category, series)` — milestone 221.
    on_point: Option<Box<dyn Fn(usize, usize) -> Msg>>,
    /// **Pinned** point `(category, series)`, highlighted by a persistent halo + ring —
    /// milestone 223.
    selected: Option<(usize, usize)>,
    /// **100%** stacking: each category is normalised to its own total (proportions) — milestone 224.
    normalized: bool,
}

impl<Msg> LineChart<Msg> {
    /// Creates a line chart from a series of `(label, value)`.
    pub fn new(data: impl IntoIterator<Item = (impl Into<String>, f32)>) -> Self {
        Self {
            values: data
                .into_iter()
                .map(|(l, v)| (l.into(), v.max(0.0)))
                .collect(),
            color: None,
            height: DEFAULT_HEIGHT,
            grid: 0,
            fill: false,
            name: None,
            extra: Vec::new(),
            legend: false,
            stacked: false,
            hidden: Vec::new(),
            on_legend: None,
            animated: false,
            on_point: None,
            selected: None,
            normalized: false,
        }
    }

    /// Overrides the colour of the stroke (default: the theme's `primary`).
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Height of the chart in logical pixels (200 by default).
    pub fn height(mut self, height: f32) -> Self {
        self.height = height.max(X_LABEL_H + VALUE_SIZE + 8.0);
        self
    }

    /// Adds a **y-axis**: `divisions` horizontal grid lines with their ticks (`0..max`) in a
    /// left margin. `0` (the default) = no axis.
    pub fn grid(mut self, divisions: usize) -> Self {
        self.grid = divisions;
        self
    }

    /// Fills the **area** under the curve (the stroke's colour heavily dimmed), to emphasise
    /// the volume rather than the trend alone. Off by default.
    pub fn area(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    /// Names the **main** series (displayed in the legend).
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Adds an **extra series** `(name, colour, values)`, aligned by index onto the main series'
    /// categories. Every series shares the scale and the axis.
    pub fn series(
        mut self,
        name: impl Into<String>,
        color: Color,
        values: impl IntoIterator<Item = f32>,
    ) -> Self {
        self.extra.push((
            name.into(),
            color,
            values.into_iter().map(|v| v.max(0.0)).collect(),
        ));
        self
    }

    /// Shows a **legend** (colour swatch + name) for each named series. Off by default.
    pub fn legend(mut self, legend: bool) -> Self {
        self.legend = legend;
        self
    }

    /// **Stacks** the series: each area is cumulated above the previous ones (cumulative areas),
    /// so a total and its composition can be read. Implies filling the bands. Off by default.
    pub fn stacked(mut self, stacked: bool) -> Self {
        self.stacked = stacked;
        self
    }

    /// **Hides** the series at the given indices (not drawn, dimmed in the legend) — milestone 215.
    pub fn hidden(mut self, indices: impl IntoIterator<Item = usize>) -> Self {
        self.hidden = indices.into_iter().collect();
        self
    }

    /// Makes the **legend clickable**: `on_legend(index)` on a click on an entry — milestone 215.
    pub fn on_legend(mut self, on_legend: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_legend = Some(Box::new(on_legend));
        self
    }

    /// Animates a **pulsing halo** (growing then fading) on the hovered point. Asks for a
    /// continuous repaint for as long as the chart is displayed. Off by default — milestone 217.
    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }

    /// Makes the **points clickable**: `on_point(category, series)` on a click near a marker
    /// (of the visible series). None by default — milestone 221.
    pub fn on_point(mut self, on_point: impl Fn(usize, usize) -> Msg + 'static) -> Self {
        self.on_point = Some(Box::new(on_point));
        self
    }

    /// **Pins** a point `(category, series)`: it gets a persistent halo + accent ring
    /// (highlighting the current selection, as a clicked detail). `None` = nothing — milestone 223.
    pub fn selected(mut self, selected: Option<(usize, usize)>) -> Self {
        self.selected = selected;
        self
    }

    /// Normalises the stacking to **100%**: each category fills the whole height, each stratum
    /// taking its **share** of the total. Only has an effect on multi-series stacked areas.
    /// Off by default — milestone 224.
    pub fn normalized(mut self, normalized: bool) -> Self {
        self.normalized = normalized;
        self
    }

    /// Total (of the **visible** series) of category `i` — the denominator of 100% stacking.
    fn category_total(&self, i: usize) -> f32 {
        let base = if self.hidden.contains(&0) {
            0.0
        } else {
            self.values.get(i).map(|(_, v)| *v).unwrap_or(0.0)
        };
        let rest: f32 = self
            .extra
            .iter()
            .enumerate()
            .filter(|(j, _)| !self.hidden.contains(&(j + 1)))
            .map(|(_, (_, _, vs))| vs.get(i).copied().unwrap_or(0.0))
            .sum();
        (base + rest).max(1e-6)
    }

    /// Names of every series (the main one then the extras).
    fn series_names(&self) -> Vec<&str> {
        let mut names = vec![self.name.as_deref().unwrap_or("Series 1")];
        names.extend(self.extra.iter().map(|(n, _, _)| n.as_str()));
        names
    }

    /// The largest value across **all** the series (at least 1, for a stable scale).
    fn max_value(&self) -> f32 {
        let primary = self.values.iter().map(|(_, v)| *v);
        let extra = self.extra.iter().flat_map(|(_, _, vs)| vs.iter().copied());
        primary.chain(extra).fold(0.0, f32::max).max(1.0)
    }

    /// The largest **sum** of the series per category (the scale in stacked mode).
    fn stacked_max(&self) -> f32 {
        let n = self.values.len();
        (0..n)
            .map(|i| {
                let base = self.values[i].1;
                let rest: f32 = self
                    .extra
                    .iter()
                    .map(|(_, _, vs)| vs.get(i).copied().unwrap_or(0.0))
                    .sum();
                base + rest
            })
            .fold(0.0, f32::max)
            .max(1.0)
    }

    /// Should the legend be drawn (enabled **and** at least one named series)?
    fn has_legend(&self) -> bool {
        self.legend && (self.name.is_some() || !self.extra.is_empty())
    }
}

impl<Msg> Widget<Msg> for LineChart<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Percent(1.0),
            height: Dimension::Length(self.height),
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let n = self.values.len();
        if n == 0 {
            return;
        }
        let o = status.opacity;
        let accent = self.color.unwrap_or(theme.primary);
        let single = self.extra.is_empty();
        let stacked = self.stacked && !single;
        // **100%** stacking (milestone 224): each category is normalised to its own total (proportions).
        let normalized = self.normalized && stacked;
        // When stacked, the scale must hold the cumulative **total** per category.
        let max = if stacked {
            self.stacked_max()
        } else {
            self.max_value()
        };

        // Geometry shared with the BarChart: the band of values at the top (value labels, single
        // series only), the category labels at the bottom, a left margin for the axis, and — where
        // there is one — a legend band right at the top.
        let legend_h = if self.has_legend() { LEGEND_H } else { 0.0 };
        let baseline_y = bounds.y + bounds.height - X_LABEL_H;
        let plot_top = bounds.y + legend_h + VALUE_SIZE + 6.0;
        let plot_h = (baseline_y - plot_top).max(1.0);
        let axis_w = axis_width(self.grid);
        let plot_left = bounds.x + axis_w;
        let plot_w = bounds.width - axis_w;
        let slot = plot_w / n as f32;

        // Horizontal grid + ticks (behind the curves); in 100% mode the axis is in percentages.
        draw_grid(
            scene, theme, plot_left, plot_w, plot_top, baseline_y, max, self.grid, normalized, o,
        );

        // Baseline (the x-axis).
        scene.fill_rect(
            Rect::new(plot_left, baseline_y, plot_w, 1.5),
            theme.scheme.outline_variant.fade(o),
        );

        // Every series to draw: the main one then the extras, aligned by index.
        let primary: Vec<f32> = self.values.iter().map(|(_, v)| *v).collect();
        let mut series: Vec<(Color, &str, &[f32])> = vec![(
            accent,
            self.name.as_deref().unwrap_or("Series 1"),
            primary.as_slice(),
        )];
        for (name, color, vals) in &self.extra {
            series.push((*color, name.as_str(), vals.as_slice()));
        }

        // Coordinates of a value (index, value) → a screen point.
        let pt = |i: usize, v: f32| {
            Point::new(
                plot_left + slot * (i as f32 + 0.5),
                baseline_y - (v / max) * plot_h,
            )
        };
        // Point of a stacked **cumulative** value: in 100% mode the denominator is the category's
        // total (a full-height band), otherwise the global scale (milestone 224).
        let spt = |i: usize, cum: f32| {
            let denom = if normalized {
                self.category_total(i)
            } else {
                max
            };
            Point::new(
                plot_left + slot * (i as f32 + 0.5),
                baseline_y - (cum / denom) * plot_h,
            )
        };

        if stacked {
            // **Cumulative** areas: each series is a band between its lower and upper cumulative
            // values, from the bottom up; the stroke follows the upper edge.
            let mut lower = vec![0.0_f32; n];
            for (j, (color, _, vals)) in series.iter().enumerate() {
                if self.hidden.contains(&j) {
                    continue;
                }
                let upper: Vec<f32> = (0..n)
                    .map(|i| lower[i] + vals.get(i).copied().unwrap_or(0.0))
                    .collect();
                if n >= 2 {
                    // The band: lower edge (left→right) then upper edge (right→left).
                    let mut band = Path::new().move_to(spt(0, lower[0]));
                    for (i, low) in lower.iter().enumerate().take(n).skip(1) {
                        band = band.line_to(spt(i, *low));
                    }
                    for i in (0..n).rev() {
                        band = band.line_to(spt(i, upper[i]));
                    }
                    scene.fill_path(&band, color.fade(o * STACK_ALPHA));
                    // Stroke of the upper edge.
                    let mut line = Path::new().move_to(spt(0, upper[0]));
                    for (i, high) in upper.iter().enumerate().take(n).skip(1) {
                        line = line.line_to(spt(i, *high));
                    }
                    scene.stroke_path(&line, color.fade(o), LINE_W);
                }
                // The value (or the % share) at the centre of the band on each category, where the
                // band is thick enough — parity with the bar strata (milestones 227/229) —
                // milestone 230.
                for i in 0..n {
                    let value = vals.get(i).copied().unwrap_or(0.0);
                    if value <= 0.0 {
                        continue;
                    }
                    let y_lo = spt(i, lower[i]).y;
                    let y_hi = spt(i, upper[i]).y;
                    if y_lo - y_hi >= STRATA_LABEL_SIZE + 4.0 {
                        let label = if normalized {
                            format!(
                                "{}%",
                                (value / self.category_total(i) * 100.0).round() as i64
                            )
                        } else {
                            format_value(value)
                        };
                        let lw = frus_text::measure(&label, STRATA_LABEL_SIZE).width;
                        let px = plot_left + slot * (i as f32 + 0.5);
                        // Clamps the label to the plot area (edge categories would overflow otherwise).
                        let lx = (px - lw * 0.5)
                            .clamp(plot_left, (plot_left + plot_w - lw).max(plot_left));
                        scene.text(
                            Point::new(lx, (y_lo + y_hi) * 0.5 - STRATA_LABEL_SIZE * 0.5),
                            label,
                            STRATA_LABEL_SIZE,
                            theme.on_primary.fade(o * 0.95),
                        );
                    }
                }
                lower = upper;
            }
        } else {
            for (j, (color, _, vals)) in series.iter().enumerate() {
                if self.hidden.contains(&j) {
                    continue;
                }
                let points: Vec<Point> = (0..n.min(vals.len())).map(|i| pt(i, vals[i])).collect();
                // Area under the curve (single series only, a closed non-zero path).
                if single && self.fill && points.len() >= 2 {
                    let mut area = Path::new().move_to(Point::new(points[0].x, baseline_y));
                    for p in &points {
                        area = area.line_to(*p);
                    }
                    area = area.line_to(Point::new(points[points.len() - 1].x, baseline_y));
                    scene.fill_path(&area, color.fade(o * AREA_ALPHA));
                }
                // Polyligne.
                if points.len() >= 2 {
                    let mut line = Path::new().move_to(points[0]);
                    for p in &points[1..] {
                        line = line.line_to(*p);
                    }
                    scene.stroke_path(&line, color.fade(o), LINE_W);
                }
                // Markers, and — for a single series — the value above each point.
                for (i, p) in points.iter().enumerate() {
                    scene.fill_path(&Path::circle(*p, MARKER_R), color.fade(o));
                    if single {
                        let vs = format_value(vals[i]);
                        let vw = frus_text::measure(&vs, VALUE_SIZE).width;
                        scene.text(
                            Point::new(p.x - vw * 0.5, p.y - MARKER_R - VALUE_SIZE - 2.0),
                            vs,
                            VALUE_SIZE,
                            theme.on_surface.fade(o),
                        );
                    }
                }
            }
        }

        // Pinned point (milestone 223): a persistent halo + accent ring on the selected marker
        // (not when stacked, and not on a hidden series), independent of hover.
        if let Some((sc, ss)) = self.selected {
            if !stacked && ss < series.len() && !self.hidden.contains(&ss) && sc < n {
                let (color, _, vals) = series[ss];
                if let Some(&v) = vals.get(sc) {
                    let p = pt(sc, v);
                    scene.fill_path(&Path::circle(p, MARKER_R + 6.0), color.fade(o * 0.22));
                    scene.stroke_path(&Path::circle(p, MARKER_R + 3.0), color.fade(o), 2.0);
                }
            }
        }

        // Category labels (once, below the baseline).
        for (i, (label, _)) in self.values.iter().enumerate() {
            let lw = frus_text::measure(label, LABEL_SIZE).width;
            scene.text(
                Point::new(
                    plot_left + slot * (i as f32 + 0.5) - lw * 0.5,
                    baseline_y + 4.0,
                ),
                label.clone(),
                LABEL_SIZE,
                theme.muted.fade(o),
            );
        }

        // Legend (the top band), shared; hidden series are dimmed in it.
        if self.has_legend() {
            let entries: Vec<(Color, &str)> = series
                .iter()
                .enumerate()
                .map(|(i, (c, n, _))| {
                    (
                        if self.hidden.contains(&i) {
                            c.fade(0.35)
                        } else {
                            *c
                        },
                        *n,
                    )
                })
                .collect();
            draw_legend(scene, theme, plot_left, bounds.y, &entries, o);
        }

        // Sub-region tooltip (milestone 211): when the pointer hovers the plot area
        // (`hover_cursor`, set by the shell through `cursor_icon`), the nearest category is brought
        // forward, each visible series' marker is accented, and their values are listed.
        if let Some(hc) = status.hover_cursor {
            let lx = hc.x - bounds.x;
            let hi =
                (((lx - plot_left) / slot - 0.5).round() as i64).clamp(0, n as i64 - 1) as usize;
            let gx = plot_left + slot * (hi as f32 + 0.5);
            let mut lines: Vec<(Option<Color>, String)> = vec![(None, self.values[hi].0.clone())];
            // In 100% mode each measure is followed by its share of the hovered category's total (milestone 226).
            let percent_of = if normalized {
                Some(self.category_total(hi))
            } else {
                None
            };
            for (j, (color, name, vals)) in series.iter().enumerate() {
                if self.hidden.contains(&j) || hi >= vals.len() {
                    continue;
                }
                // The marker accented at the value (not when stacked: an individual height means
                // nothing on a cumulative stratum).
                if !stacked {
                    let py = baseline_y - (vals[hi] / max) * plot_h;
                    // Animated pulsing halo (milestone 217): grows then fades under the marker.
                    if self.animated {
                        let phase = (status.time * PULSE_SPEED).fract();
                        let r = (MARKER_R + 2.0) + phase * PULSE_GROW;
                        scene.fill_path(
                            &Path::circle(Point::new(gx, py), r),
                            color.fade(o * (1.0 - phase) * 0.4),
                        );
                    }
                    scene.fill_path(
                        &Path::circle(Point::new(gx, py), MARKER_R + 2.0),
                        color.fade(o),
                    );
                }
                let vtxt = format_measure(vals[hi], percent_of);
                let txt = if single {
                    vtxt
                } else {
                    format!("{}  {}", name, vtxt)
                };
                lines.push((Some(*color), txt));
            }
            draw_tooltip(scene, theme, bounds, gx, plot_top, baseline_y, &lines, o);
        }
    }

    fn cursor_icon(
        &self,
        local_x: f32,
        local_y: f32,
        width: f32,
        height: f32,
    ) -> Option<crate::interaction::Cursor> {
        if self.values.is_empty() {
            return None;
        }
        let legend_h = if self.has_legend() { LEGEND_H } else { 0.0 };
        let plot_top = legend_h + VALUE_SIZE + 6.0;
        chart_plot_hit(
            local_x,
            local_y,
            width,
            height,
            axis_width(self.grid),
            plot_top,
        )
    }

    fn positional_click(&self, local_x: f32, local_y: f32, width: f32, height: f32) -> Option<Msg> {
        // 1) A click on a legend entry → on_legend(index) (milestone 215).
        if let Some(f) = &self.on_legend {
            if self.has_legend() {
                if let Some(idx) = legend_hit(
                    local_x,
                    local_y,
                    axis_width(self.grid),
                    &self.series_names(),
                ) {
                    return Some(f(idx));
                }
            }
        }
        // 2) A click on a **point** → on_point(category, series) (milestone 221). Not in stacked
        // mode, where individual markers do not exist. The geometry matches the paint.
        let f = self.on_point.as_ref()?;
        let n = self.values.len();
        if n == 0 || (self.stacked && !self.extra.is_empty()) {
            return None;
        }
        let legend_h = if self.has_legend() { LEGEND_H } else { 0.0 };
        let baseline_y = height - X_LABEL_H;
        let plot_top = legend_h + VALUE_SIZE + 6.0;
        let plot_h = (baseline_y - plot_top).max(1.0);
        let plot_left = axis_width(self.grid);
        let slot = (width - plot_left) / n as f32;
        let max = self.max_value();
        let primary: Vec<f32> = self.values.iter().map(|(_, v)| *v).collect();
        for j in 0..(1 + self.extra.len()) {
            if self.hidden.contains(&j) {
                continue;
            }
            let vals: &[f32] = if j == 0 {
                &primary
            } else {
                &self.extra[j - 1].2
            };
            for (i, &v) in vals.iter().enumerate().take(n) {
                let px = plot_left + slot * (i as f32 + 0.5);
                let py = baseline_y - (v / max) * plot_h;
                if (local_x - px).hypot(local_y - py) <= POINT_HIT_R {
                    return Some(f(i, j));
                }
            }
        }
        None
    }

    fn continuous(&self) -> bool {
        // Continuous repaint while the pulsing halo is on (milestone 217).
        self.animated
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frus_core::Primitive;

    fn paint_chart(chart: &BarChart, w: f32, h: f32) -> Vec<Primitive> {
        let mut scene = Scene::new();
        Widget::<()>::paint(
            chart,
            Rect::new(0.0, 0.0, w, h),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        scene.primitives().to_vec()
    }

    #[test]
    fn value_formatting() {
        assert_eq!(format_value(3.0), "3");
        assert_eq!(format_value(2.5), "2.5");
    }

    #[test]
    fn empty_series_paints_nothing() {
        assert!(paint_chart(&BarChart::new(Vec::<(String, f32)>::new()), 300.0, 200.0).is_empty());
    }

    #[test]
    fn bars_scale_to_the_max_value() {
        // Three bars: the largest value gives the tallest bar.
        let chart = BarChart::new([("A", 2.0), ("B", 6.0), ("C", 4.0)]);
        let prims = paint_chart(&chart, 300.0, 200.0);
        // Rects = the baseline + 3 bars.
        let bar_heights: Vec<f32> = prims
            .iter()
            .filter_map(|p| match p {
                // Bars are taller than 2 (the baseline is 1.5 tall).
                Primitive::Rect { rect, .. } if rect.height > 2.0 => Some(rect.height),
                _ => None,
            })
            .collect();
        assert_eq!(bar_heights.len(), 3, "one bar per value");
        // B (6) is the tallest; A (2) the shortest; proportional.
        let max_h = bar_heights.iter().cloned().fold(0.0_f32, f32::max);
        let min_h = bar_heights.iter().cloned().fold(f32::MAX, f32::min);
        assert!(
            max_h > min_h * 2.5,
            "6 is three times 2: {max_h} vs {min_h}"
        );
        // Values and labels drawn.
        let has_text = |t: &str| {
            prims
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(
            has_text("6") && has_text("2") && has_text("4"),
            "values displayed"
        );
        assert!(
            has_text("A") && has_text("B") && has_text("C"),
            "labels displayed"
        );
    }

    #[test]
    fn grouped_series_draw_a_bar_per_series_and_a_legend() {
        let chart = BarChart::new([("A", 4.0), ("B", 8.0), ("C", 6.0)])
            .name("This year")
            .series("Last year", Color::rgb8(200, 120, 80), [3.0, 7.0, 5.0])
            .legend(true);
        let prims = paint_chart(&chart, 320.0, 220.0);
        // 3 categories × 2 series = 6 bars (tall ones; > the baseline and the swatches).
        let bars = prims
            .iter()
            .filter(|p| matches!(p, Primitive::Rect { rect, .. } if rect.height > 15.0))
            .count();
        assert_eq!(bars, 6, "one bar per (category, series)");
        // Two legend swatches (~10×10).
        let swatches = prims
            .iter()
            .filter(|p| matches!(p, Primitive::Rect { rect, .. }
                if (rect.width - LEGEND_SWATCH).abs() < 0.5 && (rect.height - LEGEND_SWATCH).abs() < 0.5))
            .count();
        assert_eq!(swatches, 2, "one swatch per series");
        let has_text = |t: &str| {
            prims
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(
            has_text("This year") && has_text("Last year"),
            "series names in the legend"
        );
    }

    #[test]
    fn stacked_bars_share_one_column_per_category() {
        let chart = BarChart::new([("A", 2.0), ("B", 4.0)])
            .series("x", Color::rgb8(1, 2, 3), [3.0, 1.0])
            .stacked(true);
        // Stacked scale: the largest total = max(2+3, 4+1) = 5.
        assert_eq!(chart.stacked_max(), 5.0);
        let prims = paint_chart(&chart, 300.0, 200.0);
        let seg_widths: Vec<f32> = prims
            .iter()
            .filter_map(|p| match p {
                Primitive::Rect { rect, .. } if rect.height > 2.0 => Some(rect.width),
                _ => None,
            })
            .collect();
        // 2 categories × 2 series = 4 segments, all at the group's full width (one column).
        assert_eq!(seg_widths.len(), 4, "one segment per (category, series)");
        let group_w = (300.0 / 2.0) * BAR_FILL;
        assert!(
            seg_widths.iter().all(|w| (w - group_w).abs() < 0.5),
            "stacked segments = full width, got {seg_widths:?}"
        );
    }

    #[test]
    fn normalized_stacked_bars_fill_each_column() {
        // 100% mode: each column fills the whole height, whatever its raw sum.
        let make = |norm: bool| {
            BarChart::new([("A", 2.0), ("B", 4.0)])
                .series("x", Color::rgb8(1, 2, 3), [3.0, 4.0])
                .stacked(true)
                .normalized(norm)
        };
        // Cumulative height of column A's segments (the left half: x < 150).
        let col_a = |chart: &BarChart| {
            paint_chart(chart, 300.0, 200.0)
                .iter()
                .filter_map(|p| match p {
                    Primitive::Rect { rect, .. } if rect.height > 2.0 && rect.x < 150.0 => {
                        Some(rect.height)
                    }
                    _ => None,
                })
                .sum::<f32>()
        };
        let plot_h = (200.0 - X_LABEL_H) - (VALUE_SIZE + 6.0); // 160

        // Column A (total 5): full in 100% mode...
        assert!(
            (col_a(&make(true)) - plot_h).abs() < 1.0,
            "column A full in 100% mode, got {}",
            col_a(&make(true))
        );
        // ...but partial in absolute mode (the largest total, 8, is in B).
        assert!(
            col_a(&make(false)) < plot_h - 20.0,
            "column A partial in absolute mode, got {}",
            col_a(&make(false))
        );
        // In 100% mode the axis shows a percentage.
        let prims = paint_chart(&make(true).grid(4), 300.0, 200.0);
        assert!(
            prims
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == "100%")),
            "the axis in percentages"
        );
    }

    #[test]
    fn normalized_bar_tooltip_shows_percentages() {
        // Two stacked series; in 100% mode the hover tooltip adds each series' share (%).
        let make = |norm: bool| {
            BarChart::new([("A", 2.0), ("B", 6.0)])
                .series("x", Color::rgb8(1, 2, 3), [2.0, 2.0])
                .stacked(true)
                .normalized(norm)
        };
        let tooltip_texts = |chart: &BarChart| -> Vec<String> {
            let mut scene = Scene::new();
            // Hovering category A (x ~ the centre of the 1st column).
            let status = Status {
                hover_cursor: Some(Point::new(75.0, 90.0)),
                ..Default::default()
            };
            Widget::<()>::paint(
                chart,
                Rect::new(0.0, 0.0, 300.0, 200.0),
                status,
                &Theme::default(),
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    Primitive::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect()
        };
        // Category A: both series are worth 2 → 50% each.
        let norm = tooltip_texts(&make(true));
        assert!(
            norm.iter().any(|t| t.contains("(50%)")),
            "a % share in the 100%-mode tooltip, got {norm:?}"
        );
        // In absolute mode: no mention of a percentage.
        let abs = tooltip_texts(&make(false));
        assert!(
            !abs.iter().any(|t| t.contains('%')),
            "no % in absolute mode, got {abs:?}"
        );
    }

    #[test]
    fn normalized_bars_label_each_strata_with_its_percentage() {
        // In 100% mode every stratum tall enough carries its share (%) at its centre.
        let make = |norm: bool| {
            BarChart::new([("A", 2.0), ("B", 6.0)])
                .series("x", Color::rgb8(1, 2, 3), [2.0, 2.0])
                .stacked(true)
                .normalized(norm)
        };
        // Without an axis (so no % ticks), any text ending in "%" is a stratum label.
        let pct_labels = |chart: &BarChart| {
            paint_chart(chart, 300.0, 260.0)
                .iter()
                .filter(|p| matches!(p, Primitive::Text { text, .. } if text.ends_with('%')))
                .count()
        };
        // 2 categories × 2 visible series = 4 labelled strata.
        assert_eq!(pct_labels(&make(true)), 4, "one % share per stratum");
        assert_eq!(pct_labels(&make(false)), 0, "no % in absolute mode");
    }

    #[test]
    fn stacked_absolute_bars_show_the_column_total() {
        let make = || {
            BarChart::new([("A", 2.0), ("B", 4.0)]).series("x", Color::rgb8(1, 2, 3), [3.0, 1.0])
        };
        // Absolute stacking: each column's total (A = 2+3 = 5, B = 4+1 = 5) is written above it.
        let abs = paint_chart(&make().stacked(true), 300.0, 220.0);
        let count5 = abs
            .iter()
            .filter(|p| matches!(p, Primitive::Text { text, .. } if text == "5"))
            .count();
        assert_eq!(count5, 2, "one total per column (5 and 5)");
        // In 100% mode: no raw total (full columns, shares in %).
        let norm = paint_chart(&make().stacked(true).normalized(true), 300.0, 220.0);
        assert!(
            !norm
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == "5")),
            "no raw total in 100% mode"
        );
    }

    #[test]
    fn stacked_absolute_bars_label_each_strata_with_its_value() {
        // Absolute stacking: every stratum tall enough carries its raw value (parity with the % in 100% mode).
        let chart = BarChart::new([("A", 3.0), ("B", 5.0)])
            .series("x", Color::rgb8(1, 2, 3), [4.0, 6.0])
            .stacked(true);
        let prims = paint_chart(&chart, 300.0, 260.0);
        let has = |t: &str| {
            prims
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        // Stratum values: A = 3 and 4, B = 5 and 6.
        assert!(
            has("3") && has("4") && has("6"),
            "every stratum carries its value"
        );
        // The column total (A = 7, B = 11) stays at the top (milestone 228).
        assert!(has("7") && has("11"), "column total kept");
    }

    #[test]
    fn hovering_bars_shows_a_tooltip_guide() {
        use crate::interaction::Cursor;
        let chart = BarChart::new([("A", 2.0), ("B", 6.0), ("C", 4.0)]);
        let guides = |status: Status| {
            let mut scene = Scene::new();
            Widget::<()>::paint(
                &chart,
                Rect::new(0.0, 0.0, 300.0, 220.0),
                status,
                &Theme::default(),
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .filter(|p| matches!(p, Primitive::Rect { rect, .. } if rect.width <= 1.5 && rect.height > 50.0))
                .count()
        };
        let hovering = Status {
            hover_cursor: Some(Point::new(150.0, 100.0)),
            ..Default::default()
        };
        assert_eq!(guides(hovering), 1, "one guide when the area is hovered");
        assert_eq!(guides(Status::default()), 0, "no tooltip without hover");
        assert_eq!(
            Widget::<()>::cursor_icon(&chart, 150.0, 100.0, 300.0, 220.0),
            Some(Cursor::Default)
        );
        assert_eq!(
            Widget::<()>::cursor_icon(&chart, 150.0, 5.0, 300.0, 220.0),
            None
        );
    }

    #[test]
    fn clicking_a_bar_emits_category_and_series() {
        // Two grouped series: 2 bars per category. The geometry (width 300, height 200, no axis
        // and no legend) matches the paint.
        let chart = BarChart::<(usize, usize)>::new([("A", 2.0), ("B", 6.0)])
            .series("x", Color::rgb8(1, 2, 3), [4.0, 1.0])
            .on_point(|c, s| (c, s));
        let baseline_y = 200.0 - X_LABEL_H;
        let plot_h = baseline_y - (VALUE_SIZE + 6.0);
        let slot = 300.0 / 2.0;
        let group_w = slot * BAR_FILL;
        let bar_w = group_w / 2.0;
        // Category A (i=0), extra series (j=1), value 4 (max = 6).
        let cx = slot * 0.5;
        let group_left = cx - group_w * 0.5;
        let bx = group_left + bar_w + (bar_w - bar_w * 0.86) * 0.5;
        let bar_cx = bx + bar_w * 0.86 * 0.5;
        let mid_y = baseline_y - (4.0 / 6.0) * plot_h * 0.5;
        assert_eq!(
            Widget::<(usize, usize)>::positional_click(&chart, bar_cx, mid_y, 300.0, 200.0),
            Some((0, 1)),
            "a click in the middle of category A's 2nd bar"
        );
        // Above the bar (empty space): no message.
        assert_eq!(
            Widget::<(usize, usize)>::positional_click(&chart, bar_cx, 2.0, 300.0, 200.0),
            None
        );
        // Stacked stratum: a click in the column returns the stratum that was hit.
        let stacked = BarChart::<(usize, usize)>::new([("A", 2.0), ("B", 6.0)])
            .series("x", Color::rgb8(1, 2, 3), [4.0, 1.0])
            .stacked(true)
            .on_point(|c, s| (c, s));
        // Category A: stratum 0 (value 2) at the bottom, stratum 1 (value 4) above; max total = 6.
        let low_y = baseline_y - (1.0 / 6.0) * plot_h; // inside the bottom stratum (0..2)
        assert_eq!(
            Widget::<(usize, usize)>::positional_click(&stacked, cx, low_y, 300.0, 200.0),
            Some((0, 0)),
            "a click low in the column = stratum 0"
        );
        // A hidden extra series: its bar is no longer clickable.
        let hidden = BarChart::<(usize, usize)>::new([("A", 2.0), ("B", 6.0)])
            .series("x", Color::rgb8(1, 2, 3), [4.0, 1.0])
            .on_point(|c, s| (c, s))
            .hidden([1]);
        assert_eq!(
            Widget::<(usize, usize)>::positional_click(&hidden, bar_cx, mid_y, 300.0, 200.0),
            None,
            "a hidden series' bar: no click"
        );
    }

    #[test]
    fn selected_bar_draws_a_persistent_ring() {
        // The pinned bar gets a **stroked** rect (ordinary bars have a 0-width border).
        let rings = |chart: &BarChart| {
            paint_chart(chart, 300.0, 200.0)
                .iter()
                .filter(
                    |p| matches!(p, Primitive::Rect { border_width, .. } if *border_width > 0.5),
                )
                .count()
        };
        assert_eq!(
            rings(&BarChart::new([("A", 2.0), ("B", 6.0)])),
            0,
            "no ring without a selection"
        );
        assert_eq!(
            rings(&BarChart::new([("A", 2.0), ("B", 6.0)]).selected(Some((1, 0)))),
            1,
            "one ring on the pinned bar"
        );
        // A pinned hidden series: its bar is not drawn, so there is no ring.
        let hidden = BarChart::new([("A", 2.0), ("B", 6.0)])
            .series("x", Color::rgb8(1, 2, 3), [3.0, 1.0])
            .hidden([1])
            .selected(Some((0, 1)));
        assert_eq!(rings(&hidden), 0, "no ring on a hidden series");
    }

    fn paint_line(chart: &LineChart, w: f32, h: f32) -> Vec<Primitive> {
        let mut scene = Scene::new();
        Widget::<()>::paint(
            chart,
            Rect::new(0.0, 0.0, w, h),
            Status::default(),
            &Theme::default(),
            &mut scene,
        );
        scene.primitives().to_vec()
    }

    #[test]
    fn line_empty_series_paints_nothing() {
        assert!(paint_line(&LineChart::new(Vec::<(String, f32)>::new()), 300.0, 200.0).is_empty());
    }

    #[test]
    fn line_connects_all_points() {
        let chart = LineChart::new([("A", 2.0), ("B", 6.0), ("C", 4.0)]);
        let prims = paint_line(&chart, 300.0, 200.0);
        // One stroked polyline (a path with a stroke, no fill).
        let polyline = prims.iter().find_map(|p| match p {
            Primitive::Path {
                path,
                stroke: Some(_),
                fill: None,
                ..
            } => Some(path),
            _ => None,
        });
        let polyline = polyline.expect("a stroked polyline");
        // move_to + 2 line_to for three points.
        let segments = polyline
            .verbs()
            .iter()
            .filter(|v| matches!(v, frus_core::PathVerb::LineTo(_)))
            .count();
        assert_eq!(segments, 2, "deux segments relient trois points");
        // One marker (a filled path) per point.
        let markers = prims
            .iter()
            .filter(|p| {
                matches!(
                    p,
                    Primitive::Path {
                        fill: Some(_),
                        stroke: None,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(markers, 3, "one marker per point");
        // Values and labels drawn.
        let has_text = |t: &str| {
            prims
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(
            has_text("6") && has_text("2") && has_text("4"),
            "values displayed"
        );
        assert!(
            has_text("A") && has_text("B") && has_text("C"),
            "labels displayed"
        );
    }

    #[test]
    fn hovering_the_plot_shows_a_tooltip_guide() {
        use crate::interaction::Cursor;
        let chart = LineChart::new([("A", 2.0), ("B", 6.0), ("C", 4.0)]);
        // Thin, tall vertical rects = the tooltip's guide.
        let guides = |status: Status| {
            let mut scene = Scene::new();
            Widget::<()>::paint(
                &chart,
                Rect::new(0.0, 0.0, 300.0, 220.0),
                status,
                &Theme::default(),
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .filter(|p| {
                    matches!(p, Primitive::Rect { rect, .. }
                    if rect.width <= 1.5 && rect.height > 50.0)
                })
                .count()
        };
        let hovering = Status {
            hover_cursor: Some(Point::new(150.0, 100.0)),
            ..Default::default()
        };
        assert_eq!(
            guides(hovering),
            1,
            "one vertical guide when the area is hovered"
        );
        assert_eq!(guides(Status::default()), 0, "no tooltip without hover");
        // `cursor_icon` turns tracking on over the plot area (Default), not outside it.
        assert_eq!(
            Widget::<()>::cursor_icon(&chart, 150.0, 100.0, 300.0, 220.0),
            Some(Cursor::Default)
        );
        assert_eq!(
            Widget::<()>::cursor_icon(&chart, 150.0, 5.0, 300.0, 220.0),
            None,
            "above the area"
        );
    }

    #[test]
    fn multi_series_draws_each_line_and_a_legend() {
        let chart = LineChart::new([("A", 2.0), ("B", 6.0), ("C", 4.0)])
            .name("Sales")
            .series(
                "Costs",
                frus_core::Color::rgb8(200, 80, 80),
                [1.0, 3.0, 2.0],
            )
            .legend(true);
        let prims = paint_line(&chart, 300.0, 220.0);
        // Two polylines (one per series).
        let polylines = prims
            .iter()
            .filter(|p| {
                matches!(
                    p,
                    Primitive::Path {
                        stroke: Some(_),
                        fill: None,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(polylines, 2, "one polyline per series");
        // Two legend swatches (~10x10).
        let swatches = prims
            .iter()
            .filter(|p| matches!(p, Primitive::Rect { rect, .. }
                if (rect.width - LEGEND_SWATCH).abs() < 0.5 && (rect.height - LEGEND_SWATCH).abs() < 0.5))
            .count();
        assert_eq!(swatches, 2, "one swatch per series");
        // The series names appear in the legend.
        let has_text = |t: &str| {
            prims
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        assert!(
            has_text("Sales") && has_text("Costs"),
            "series names in the legend"
        );
    }

    #[test]
    fn legend_click_emits_the_series_index() {
        let chart = LineChart::<usize>::new([("A", 1.0)])
            .name("Sales")
            .series("Costs", Color::rgb8(1, 2, 3), [2.0])
            .legend(true)
            .on_legend(|i| i);
        // A click on the 1st entry (its swatch near x=0, y inside the legend band).
        assert_eq!(
            Widget::<usize>::positional_click(&chart, 5.0, 10.0, 300.0, 200.0),
            Some(0)
        );
        // 2nd entry: right after the 1st (swatch + space + "Sales" + gap).
        let after_first =
            LEGEND_SWATCH + 5.0 + frus_text::measure("Sales", LEGEND_SIZE).width + 16.0;
        assert_eq!(
            Widget::<usize>::positional_click(&chart, after_first + 4.0, 10.0, 300.0, 200.0),
            Some(1)
        );
        // Outside the band (y too low): no legend click.
        assert_eq!(
            Widget::<usize>::positional_click(&chart, 5.0, 100.0, 300.0, 200.0),
            None
        );
        // Without `on_legend` the legend is not clickable.
        let plain = LineChart::<usize>::new([("A", 1.0)])
            .name("Sales")
            .series("Costs", Color::rgb8(1, 2, 3), [2.0])
            .legend(true);
        assert_eq!(
            Widget::<usize>::positional_click(&plain, 5.0, 10.0, 300.0, 200.0),
            None
        );
    }

    #[test]
    fn animated_pulse_adds_a_halo_and_requests_continuous_repaint() {
        let animated = LineChart::new([("A", 2.0), ("B", 6.0)]).animated(true);
        let plain = LineChart::new([("A", 2.0), ("B", 6.0)]);
        assert!(
            Widget::<()>::continuous(&animated),
            "animated => a continuous repaint"
        );
        assert!(
            !Widget::<()>::continuous(&plain),
            "static => no continuous repaint"
        );
        // Filled circles (markers/halo: filled paths with no straight segment) on hover.
        let circles = |chart: &LineChart, t: f32| {
            let mut scene = Scene::new();
            let status = Status {
                hover_cursor: Some(Point::new(150.0, 100.0)),
                time: t,
                ..Default::default()
            };
            Widget::<()>::paint(
                chart,
                Rect::new(0.0, 0.0, 300.0, 200.0),
                status,
                &Theme::default(),
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .filter(|p| {
                    matches!(p, Primitive::Path { fill: Some(_), stroke: None, path, .. }
                    if !path.verbs().iter().any(|v| matches!(v, frus_core::PathVerb::LineTo(_))))
                })
                .count()
        };
        // The animated halo adds a circle the static chart does not have (at the same hover).
        assert!(
            circles(&animated, 0.1) > circles(&plain, 0.1),
            "the animated halo adds a circle"
        );
    }

    #[test]
    fn clicking_a_point_emits_category_and_series() {
        let chart = LineChart::<(usize, usize)>::new([("A", 2.0), ("B", 6.0)])
            .series("x", Color::rgb8(1, 2, 3), [4.0, 1.0])
            .on_point(|c, s| (c, s));
        // The geometry (width 300, height 200, no axis and no legend) — matches the paint.
        let baseline_y = 200.0 - X_LABEL_H;
        let plot_h = baseline_y - (VALUE_SIZE + 6.0);
        let slot = 300.0 / 2.0;
        let px = slot * 0.5; // category A (i = 0)
        let py_primary = baseline_y - (2.0 / 6.0) * plot_h; // series 0, value 2 (max = 6)
        assert_eq!(
            Widget::<(usize, usize)>::positional_click(&chart, px, py_primary, 300.0, 200.0),
            Some((0, 0)),
            "a click on point A of the main series"
        );
        // Far from every marker: no message.
        assert_eq!(
            Widget::<(usize, usize)>::positional_click(&chart, slot, 5.0, 300.0, 200.0),
            None
        );
        // A hidden main series: its point is no longer clickable.
        let hidden = LineChart::<(usize, usize)>::new([("A", 2.0), ("B", 6.0)])
            .series("x", Color::rgb8(1, 2, 3), [4.0, 1.0])
            .on_point(|c, s| (c, s))
            .hidden([0]);
        assert_eq!(
            Widget::<(usize, usize)>::positional_click(&hidden, px, py_primary, 300.0, 200.0),
            None,
            "a hidden series' point: no click"
        );
    }

    #[test]
    fn selected_point_draws_a_persistent_ring() {
        // A ring (a **stroked** circle, no straight segment) appears on the pinned point, without hover.
        let rings = |chart: &LineChart| {
            paint_line(chart, 300.0, 200.0)
                .iter()
                .filter(|p| {
                    matches!(p, Primitive::Path { stroke: Some(_), fill: None, path, .. }
                    if !path.verbs().iter().any(|v| matches!(v, frus_core::PathVerb::LineTo(_))))
                })
                .count()
        };
        assert_eq!(
            rings(&LineChart::new([("A", 2.0), ("B", 6.0)])),
            0,
            "no ring without a selection"
        );
        assert_eq!(
            rings(&LineChart::new([("A", 2.0), ("B", 6.0)]).selected(Some((0, 0)))),
            1,
            "one ring on the pinned point"
        );
        // A pinned hidden series: no ring.
        let hidden = LineChart::new([("A", 2.0), ("B", 6.0)])
            .series("x", Color::rgb8(1, 2, 3), [3.0, 1.0])
            .hidden([0])
            .selected(Some((0, 0)));
        assert_eq!(rings(&hidden), 0, "no ring on a hidden series");
    }

    #[test]
    fn hidden_series_is_not_drawn() {
        let strokes = |chart: &LineChart| {
            paint_line(chart, 300.0, 200.0)
                .iter()
                .filter(|p| {
                    matches!(
                        p,
                        Primitive::Path {
                            stroke: Some(_),
                            fill: None,
                            ..
                        }
                    )
                })
                .count()
        };
        let both = LineChart::new([("A", 2.0), ("B", 6.0)]).series(
            "x",
            Color::rgb8(200, 80, 80),
            [3.0, 1.0],
        );
        assert_eq!(strokes(&both), 2, "two visible series = two lines");
        let hidden = LineChart::new([("A", 2.0), ("B", 6.0)])
            .series("x", Color::rgb8(200, 80, 80), [3.0, 1.0])
            .hidden([1]);
        assert_eq!(strokes(&hidden), 1, "the hidden series is not drawn");
    }

    #[test]
    fn stacked_areas_fill_a_band_per_series() {
        let make = || {
            LineChart::new([("A", 2.0), ("B", 4.0)]).series(
                "x",
                Color::rgb8(200, 80, 80),
                [3.0, 1.0],
            )
        };
        // The stacked scale takes the largest total per category: max(2+3, 4+1) = 5.
        assert_eq!(make().stacked(true).stacked_max(), 5.0);
        let bands = |chart: &LineChart| {
            paint_line(chart, 300.0, 200.0)
                .iter()
                .filter(|p| {
                    matches!(p, Primitive::Path { fill: Some(_), stroke: None, path, .. }
                    if path.verbs().iter().any(|v| matches!(v, frus_core::PathVerb::LineTo(_))))
                })
                .count()
        };
        assert_eq!(
            bands(&make().stacked(true)),
            2,
            "one cumulative band per series"
        );
        assert_eq!(
            bands(&make()),
            0,
            "without stacking: no band (multi-series with no area)"
        );
    }

    #[test]
    fn normalized_stacked_areas_fill_to_the_top() {
        let make = |norm: bool| {
            LineChart::new([("A", 2.0), ("B", 4.0)])
                .series("x", Color::rgb8(1, 2, 3), [3.0, 4.0])
                .stacked(true)
                .normalized(norm)
        };
        // The y coordinates of the **upper** edge's stroke (the last band drawn = the last visible series).
        let top_line_ys = |chart: &LineChart| -> Vec<f32> {
            let prims = paint_line(chart, 300.0, 200.0);
            let path = prims
                .iter()
                .rev()
                .find_map(|p| match p {
                    Primitive::Path {
                        stroke: Some(_),
                        fill: None,
                        path,
                        ..
                    } if path
                        .verbs()
                        .iter()
                        .any(|v| matches!(v, frus_core::PathVerb::LineTo(_))) =>
                    {
                        Some(path)
                    }
                    _ => None,
                })
                .expect("an upper-edge stroke");
            path.verbs()
                .iter()
                .filter_map(|v| match v {
                    frus_core::PathVerb::MoveTo(p) | frus_core::PathVerb::LineTo(p) => Some(p.y),
                    _ => None,
                })
                .collect()
        };
        let plot_top = VALUE_SIZE + 6.0; // 18 (no legend and no axis)

        // 100% mode: the upper edge is flat, at 100% (plot_top) on every category.
        let ys = top_line_ys(&make(true));
        assert!(
            ys.iter().all(|y| (y - plot_top).abs() < 1.0),
            "upper edge at 100% everywhere, got {ys:?}"
        );
        // Absolute mode: the upper edge follows the totals (not all at plot_top).
        let ys_abs = top_line_ys(&make(false));
        assert!(
            ys_abs.iter().any(|y| (y - plot_top).abs() > 5.0),
            "the upper edge follows the totals, got {ys_abs:?}"
        );
    }

    #[test]
    fn normalized_line_tooltip_shows_percentages() {
        // Stacked areas; in 100% mode the hover tooltip adds each series' share (%).
        let make = |norm: bool| {
            LineChart::new([("A", 2.0), ("B", 6.0)])
                .series("x", Color::rgb8(1, 2, 3), [2.0, 2.0])
                .stacked(true)
                .normalized(norm)
        };
        let tooltip_texts = |chart: &LineChart| -> Vec<String> {
            let mut scene = Scene::new();
            let status = Status {
                hover_cursor: Some(Point::new(75.0, 90.0)),
                ..Default::default()
            };
            Widget::<()>::paint(
                chart,
                Rect::new(0.0, 0.0, 300.0, 200.0),
                status,
                &Theme::default(),
                &mut scene,
            );
            scene
                .primitives()
                .iter()
                .filter_map(|p| match p {
                    Primitive::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect()
        };
        let norm = tooltip_texts(&make(true));
        assert!(
            norm.iter().any(|t| t.contains("(50%)")),
            "a % share in the 100%-mode tooltip, got {norm:?}"
        );
        let abs = tooltip_texts(&make(false));
        assert!(
            !abs.iter().any(|t| t.contains('%')),
            "no % in absolute mode, got {abs:?}"
        );
    }

    #[test]
    fn stacked_areas_label_each_band_with_value_or_percentage() {
        // Absolute mode: every band thick enough carries its value on each category.
        let abs = LineChart::new([("A", 3.0), ("B", 5.0)])
            .series("x", Color::rgb8(1, 2, 3), [4.0, 6.0])
            .stacked(true);
        let prims = paint_line(&abs, 300.0, 260.0);
        let has = |t: &str| {
            prims
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        // Band values: series 0 = 3 and 5; series x = 4 and 6.
        assert!(
            has("3") && has("4") && has("5") && has("6"),
            "band values displayed"
        );
        // 100% mode: shares in %.
        let norm = LineChart::new([("A", 2.0), ("B", 6.0)])
            .series("x", Color::rgb8(1, 2, 3), [2.0, 2.0])
            .stacked(true)
            .normalized(true);
        let np = paint_line(&norm, 300.0, 260.0);
        assert!(
            np.iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text.ends_with('%'))),
            "% shares displayed inside the bands"
        );
    }

    #[test]
    fn max_value_spans_all_series() {
        // The scale spans the extra series (max 9 > the main series' max of 6).
        let chart = LineChart::<()>::new([("A", 2.0), ("B", 6.0)]).series(
            "x",
            Color::rgb8(1, 2, 3),
            [9.0, 1.0],
        );
        assert_eq!(chart.max_value(), 9.0);
    }

    #[test]
    fn area_fills_a_polygon_under_the_curve() {
        // A **filled** path (no stroke) made of straight segments = the area; without `.area`
        // only the markers (circles, with no `LineTo`) are filled.
        let filled_polygons = |chart: &LineChart| {
            paint_line(chart, 300.0, 200.0)
                .iter()
                .filter(|p| match p {
                    Primitive::Path {
                        fill: Some(_),
                        stroke: None,
                        path,
                        ..
                    } => path
                        .verbs()
                        .iter()
                        .any(|v| matches!(v, frus_core::PathVerb::LineTo(_))),
                    _ => false,
                })
                .count()
        };
        assert_eq!(
            filled_polygons(&LineChart::new([("A", 2.0), ("B", 6.0)])),
            0,
            "no area by default"
        );
        assert_eq!(
            filled_polygons(&LineChart::new([("A", 2.0), ("B", 6.0)]).area(true)),
            1,
            "one filled area under the curve"
        );
    }

    #[test]
    fn grid_draws_horizontal_lines_and_axis_labels() {
        // A series with a max of 8 and four divisions → ticks 0, 2, 4, 6, 8.
        let chart = LineChart::new([("A", 2.0), ("B", 8.0)]).grid(4);
        let prims = paint_line(&chart, 300.0, 200.0);
        // Thin horizontal lines (height ~1): 4 grid lines + the baseline (1.5).
        let thin_lines = prims
            .iter()
            .filter(|p| matches!(p, Primitive::Rect { rect, .. } if rect.height <= 1.6))
            .count();
        assert!(
            thin_lines >= 5,
            "4 grid lines + the baseline, got {thin_lines}"
        );
        let has_text = |t: &str| {
            prims
                .iter()
                .any(|p| matches!(p, Primitive::Text { text, .. } if text == t))
        };
        // Axis ticks: 0 (the baseline) and 8 (the top) at least.
        assert!(has_text("0") && has_text("8"), "y-axis ticks");
    }

    #[test]
    fn no_grid_by_default_keeps_full_width() {
        // Without a grid, no "0" tick is drawn (the original behaviour).
        let chart = LineChart::new([("A", 2.0), ("B", 8.0)]);
        let prims = paint_line(&chart, 300.0, 200.0);
        let has_zero = prims
            .iter()
            .any(|p| matches!(p, Primitive::Text { text, .. } if text == "0"));
        assert!(!has_zero, "no axis by default");
    }
}
