//! [`DatePicker`]: a **controlled** month calendar, built on [`crate::GridView`].
//! Date arithmetic is **home-grown**, with no time dependency.

use frus_core::{Color, Point, Rect, ResolvedTextStyle, Scene, TextStyle};
use frus_layout::{Align, Dimension, FlexDirection, Style};

use crate::flex::Flex;
use crate::grid::GridView;
use crate::interaction::Status;
use crate::text::Text;
use crate::theme::Theme;
use crate::widget::Widget;

const CELL: f32 = 34.0;

/// A day's style: what the calendar was told, else what the theme says, else the
/// reference's — a Material 3 date picker sets its days in `bodyLarge`.
///
/// **Resolved once** so that the number the cell is measured with is the number the figures
/// are drawn at. Resolving is the single place the reader's font setting is applied
/// (milestone 403).
fn day_style(theme: Option<&Theme>) -> ResolvedTextStyle {
    theme
        .and_then(|t| t.widgets.date_picker.day_text_style)
        .unwrap_or_else(|| crate::theme::type_scale(theme).body_large)
        .resolved()
}

/// The weekday initial's style — the reference says `bodyLarge` for these too. See
/// [`day_style`].
fn weekday_style(theme: Option<&Theme>) -> ResolvedTextStyle {
    theme
        .and_then(|t| t.widgets.date_picker.weekday_text_style)
        .unwrap_or_else(|| crate::theme::type_scale(theme).body_large)
        .resolved()
}

/// A cell's side: the constant, unless the reader asked for figures that do not fit in it.
fn cell(theme: Option<&Theme>) -> f32 {
    frus_text::line_box(CELL, &day_style(theme), 0.0)
}
/// The gap between the two months of a dual calendar.
const DUAL_GAP: f32 = 24.0;

/// True if `year` is a leap year.
fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Number of days in month `month` (1..=12) of `year`.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// The weekday (`0` = Sunday … `6` = Saturday) of the 1st of the month (Sakamoto).
fn first_weekday(year: i32, month: u32) -> usize {
    const T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    let m = month as usize;
    (((y + y / 4 - y / 100 + y / 400 + T[m - 1] + 1) % 7 + 7) % 7) as usize
}

/// A day cell's state with respect to the selection.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DayMark {
    /// Neither selected nor within a range.
    Off,
    /// A selected day (simple mode).
    Selected,
    /// A range's **start** bound.
    Start,
    /// A range's **end** bound.
    End,
    /// A day **between** the two bounds (the connecting band).
    Between,
}

/// The range marker of a `date` `(year, month, day)` with respect to `[start, end]`:
/// a start or end bound, an in-between day, or outside the range. Dates compare as
/// tuples (year, then month, then day), so chronological order is the order of `<`.
fn range_mark(
    date: (i32, u32, u32),
    start: Option<(i32, u32, u32)>,
    end: Option<(i32, u32, u32)>,
) -> DayMark {
    match (start, end) {
        (Some(s), _) if date == s => DayMark::Start,
        (_, Some(e)) if date == e => DayMark::End,
        (Some(s), Some(e)) if s < date && date < e => DayMark::Between,
        _ => DayMark::Off,
    }
}

/// A clickable day cell (empty when `day == 0`).
struct Day<Msg> {
    day: u32,
    mark: DayMark,
    /// A **disabled** day (outside the bounds): dimmed, not clickable — milestone 231.
    disabled: bool,
    message: Option<Msg>,
}

impl<Msg> Day<Msg> {
    fn sizing(&self, theme: Option<&Theme>) -> Style {
        let side = cell(theme);
        Style {
            width: Dimension::Length(side),
            height: Dimension::Length(side),
            ..Default::default()
        }
    }
}

impl<Msg: Clone> Widget<Msg> for Day<Msg> {
    fn style(&self) -> Style {
        self.sizing(None)
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        self.sizing(Some(theme))
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        if self.day == 0 {
            return; // a filler cell
        }
        let o = status.opacity;
        // A disabled day (outside the bounds): a dimmed figure, no background, no band.
        if self.disabled {
            let label = self.day.to_string();
            let style = day_style(Some(theme));
            let w = frus_text::measure_resolved(&label, &style).width;
            scene.text(
                Point::new(
                    bounds.x + (bounds.width - w) * 0.5,
                    bounds.y + (bounds.height - style.line_height()) * 0.5,
                ),
                label,
                &style,
                theme.muted.fade(o * 0.4),
            );
            return;
        }
        // The range band (a soft background, square corners so neighbouring cells touch).
        let band = theme.primary.fade(0.18 * o);
        let half = Rect::new(bounds.x, bounds.y, bounds.width * 0.5, bounds.height);
        let right = Rect::new(
            bounds.x + bounds.width * 0.5,
            bounds.y,
            bounds.width * 0.5,
            bounds.height,
        );
        match self.mark {
            // The bound paints its half-band on the inner side to meet the in-between days.
            DayMark::Start => scene.draw_rect(right, band, 0.0, 0.0, Color::TRANSPARENT),
            DayMark::End => scene.draw_rect(half, band, 0.0, 0.0, Color::TRANSPARENT),
            DayMark::Between => scene.draw_rect(bounds, band, 0.0, 0.0, Color::TRANSPARENT),
            _ => {}
        }
        // The cell's background (a solid pill for a bound or selection, hover otherwise).
        let (bg, fg) = match self.mark {
            DayMark::Selected | DayMark::Start | DayMark::End => (theme.primary, theme.on_primary),
            _ => {
                let hovered = theme.state_layer(theme.surface, theme.on_surface, &status);
                (hovered, theme.on_surface)
            }
        };
        if self.mark != DayMark::Between {
            scene.draw_rect(bounds, bg.fade(o), CELL * 0.5, 0.0, Color::TRANSPARENT);
        }
        let label = self.day.to_string();
        let style = day_style(Some(theme));
        let w = frus_text::measure_resolved(&label, &style).width;
        scene.text(
            Point::new(
                bounds.x + (bounds.width - w) * 0.5,
                bounds.y + (bounds.height - style.line_height()) * 0.5,
            ),
            label,
            &style,
            fg.fade(o),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        self.message.clone()
    }

    fn focusable(&self) -> bool {
        self.message.is_some()
    }
}

/// A month calendar (or a pair of months in [`range_dual`](DatePicker::range_dual) mode).
pub struct DatePicker<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
    /// Two months side by side, which doubles its width.
    dual: bool,
    day_text_style: Option<TextStyle>,
    weekday_text_style: Option<TextStyle>,
}

impl<Msg> DatePicker<Msg> {
    /// The days' type, over the theme's and the reference's.
    ///
    /// It reaches the cells through [`Widget::theme_override`] rather than through the
    /// cells' fields: a calendar is assembled by five different constructors, and a value
    /// carried down the theme arrives at every one of them without any of them being
    /// taught to pass it on.
    #[must_use]
    pub fn day_text_style(mut self, style: TextStyle) -> Self {
        self.day_text_style = Some(style);
        self
    }

    /// The weekday initials' type, over the theme's and the reference's. See
    /// [`day_text_style`](Self::day_text_style).
    #[must_use]
    pub fn weekday_text_style(mut self, style: TextStyle) -> Self {
        self.weekday_text_style = Some(style);
        self
    }
}

/// **How far into the week the 1st of the month falls**, counting from whichever day
/// the week starts on where the reader is (`date.dart:356`).
///
/// [`first_weekday`] answers Sunday-based, and so does
/// [`Localizations::first_day_of_week_index`](crate::Localizations::first_day_of_week_index)
/// — so the difference between them is the number of blank cells, and it is taken
/// **Euclidean** because the difference goes negative for every locale that does not
/// start on Sunday.
///
/// This used to be `first_weekday` alone, which is the same number only in a country
/// whose week starts on Sunday. Everywhere else the days were in the wrong columns.
fn lead_cells(year: i32, month: u32, first_day_of_week: usize) -> usize {
    (first_weekday(year, month) as isize - first_day_of_week as isize).rem_euclid(7) as usize
}

impl<Msg: Clone + 'static> DatePicker<Msg> {
    /// Creates a calendar for `year`/`month` (1..=12), with an optional `selected`
    /// day. `on_select(day)` on click; `on_nav(±1)` to change month.
    pub fn new(
        year: i32,
        month: u32,
        selected: Option<u32>,
        on_select: impl Fn(u32) -> Msg + 'static,
        on_nav: impl Fn(i32) -> Msg + 'static,
    ) -> Self {
        Self::assemble(
            year,
            month,
            on_select,
            on_nav,
            move |day| {
                if selected == Some(day) {
                    DayMark::Selected
                } else {
                    DayMark::Off
                }
            },
            |_| true,
        )
    }

    /// A simple calendar **filtered by predicate**: a day `(year, month, day)` is clickable
    /// if and only if `is_enabled(date)` is true — the others are **disabled** (dimmed, not
    /// clickable). This is the general escape hatch: isolated **blackout** days, forbidden
    /// weekends, bounds, or any combination. `selected`/`on_select`/`on_nav` as in
    /// [`new`](Self::new) — milestone 235.
    pub fn filtered(
        year: i32,
        month: u32,
        selected: Option<u32>,
        is_enabled: impl Fn((i32, u32, u32)) -> bool + 'static,
        on_select: impl Fn(u32) -> Msg + 'static,
        on_nav: impl Fn(i32) -> Msg + 'static,
    ) -> Self {
        let month = month.clamp(1, 12);
        Self::assemble(
            year,
            month,
            on_select,
            on_nav,
            move |day| {
                if selected == Some(day) {
                    DayMark::Selected
                } else {
                    DayMark::Off
                }
            },
            move |day| is_enabled((year, month, day)),
        )
    }

    /// A simple **bounded** calendar: days outside `[min, max]` (dates `(year, month, day)`,
    /// with optional and **inclusive** bounds) are **disabled** — dimmed and not clickable.
    /// Identical to [`new`](Self::new) otherwise (the `selected` day, `on_select`, `on_nav`).
    /// Useful to forbid past dates, to set a booking window, and so on — milestone 231.
    pub fn bounded(
        year: i32,
        month: u32,
        selected: Option<u32>,
        min: Option<(i32, u32, u32)>,
        max: Option<(i32, u32, u32)>,
        on_select: impl Fn(u32) -> Msg + 'static,
        on_nav: impl Fn(i32) -> Msg + 'static,
    ) -> Self {
        let month = month.clamp(1, 12);
        Self::assemble(
            year,
            month,
            on_select,
            on_nav,
            move |day| {
                if selected == Some(day) {
                    DayMark::Selected
                } else {
                    DayMark::Off
                }
            },
            move |day| {
                let date = (year, month, day);
                min.is_none_or(|m| date >= m) && max.is_none_or(|m| date <= m)
            },
        )
    }

    /// A calendar in **range mode**: it brings out the `[start, end]` interval (dates
    /// `(year, month, day)`) — the bounds as solid pills, the days between as a soft band.
    /// A single bound (`end == None`) shows just the start (a selection in progress).
    /// `on_select(day)` reports the clicked day of the month shown — the application
    /// decides whether it becomes the start or the end.
    pub fn range(
        year: i32,
        month: u32,
        start: Option<(i32, u32, u32)>,
        end: Option<(i32, u32, u32)>,
        on_select: impl Fn(u32) -> Msg + 'static,
        on_nav: impl Fn(i32) -> Msg + 'static,
    ) -> Self {
        let month = month.clamp(1, 12);
        Self::assemble(
            year,
            month,
            on_select,
            on_nav,
            move |day| range_mark((year, month, day), start, end),
            |_| true,
        )
    }

    /// A calendar in **bounded range mode**: like [`range`](Self::range), but days outside
    /// `[min, max]` (optional and **inclusive** bounds) are **disabled** — dimmed and not
    /// clickable. Combines bringing out a selected interval with an allowed input window
    /// (booking a range within an open period, say) — milestone 234.
    #[allow(clippy::too_many_arguments)]
    pub fn range_bounded(
        year: i32,
        month: u32,
        start: Option<(i32, u32, u32)>,
        end: Option<(i32, u32, u32)>,
        min: Option<(i32, u32, u32)>,
        max: Option<(i32, u32, u32)>,
        on_select: impl Fn(u32) -> Msg + 'static,
        on_nav: impl Fn(i32) -> Msg + 'static,
    ) -> Self {
        let month = month.clamp(1, 12);
        Self::assemble(
            year,
            month,
            on_select,
            on_nav,
            move |day| range_mark((year, month, day), start, end),
            move |day| {
                let date = (year, month, day);
                min.is_none_or(|m| date >= m) && max.is_none_or(|m| date <= m)
            },
        )
    }

    /// A **dual** calendar: the `year`/`month` month and the **next**, side by side, sharing
    /// the same `[start, end]` range — to enter long ranges without changing month.
    /// `on_select((year, month, day))` reports the **complete** date of the day clicked (the
    /// month is disambiguated), and `on_nav(±1)` shifts the **pair**. The range band carries
    /// on from one month to the other (dates compare as whole values — see `range_mark`).
    pub fn range_dual(
        year: i32,
        month: u32,
        start: Option<(i32, u32, u32)>,
        end: Option<(i32, u32, u32)>,
        on_select: impl Fn((i32, u32, u32)) -> Msg + 'static,
        on_nav: impl Fn(i32) -> Msg + 'static,
    ) -> Self {
        let month = month.clamp(1, 12);
        let (ny, nm) = if month == 12 {
            (year + 1, 1)
        } else {
            (year, month + 1)
        };
        // Shared between the two months.
        let on_select = std::rc::Rc::new(on_select);
        let on_nav = std::rc::Rc::new(on_nav);
        let (os1, os2) = (on_select.clone(), on_select);
        let (nv1, nv2) = (on_nav.clone(), on_nav);
        let left = DatePicker::range(
            year,
            month,
            start,
            end,
            move |d| os1((year, month, d)),
            move |n| nv1(n),
        );
        let right = DatePicker::range(
            ny,
            nm,
            start,
            end,
            move |d| os2((ny, nm, d)),
            move |n| nv2(n),
        );
        let row = Flex::row().gap(DUAL_GAP).child(left).child(right);
        Self {
            children: vec![Box::new(row)],
            dual: true,
            day_text_style: None,
            weekday_text_style: None,
        }
    }

    /// Assembles the header, the weekday row and the grid; `mark_of(day)` decides each
    /// cell's state (a simple selection, or a range).
    fn assemble(
        year: i32,
        month: u32,
        on_select: impl Fn(u32) -> Msg + 'static,
        on_nav: impl Fn(i32) -> Msg + 'static,
        mark_of: impl Fn(u32) -> DayMark,
        enabled: impl Fn(u32) -> bool,
    ) -> Self {
        let month = month.clamp(1, 12);

        // The header: ‹ Month Year ›.
        let header = Flex::row()
            .align(Align::Center)
            .gap(8.0)
            .child(
                crate::IconButton::new(crate::icons::Icons::CHEVRON_LEFT)
                    .label("Previous month")
                    .icon_size(18.0)
                    .on_press(on_nav(-1)),
            )
            .child(Flex::row().flex(1.0))
            .child(
                Text::new(format!(
                    "{} {}",
                    crate::localizations::of().months()[(month - 1) as usize],
                    year
                ))
                .size(16.0),
            )
            .child(Flex::row().flex(1.0))
            .child(
                crate::IconButton::new(crate::icons::Icons::CHEVRON_RIGHT)
                    .label("Next month")
                    .icon_size(18.0)
                    .on_press(on_nav(1)),
            );

        // The weekday row, **rotated to start where the reader's week does**
        // (`calendar_date_picker.dart:1100`). The initials themselves are always listed
        // Sunday-first, which is what lets the index be an index into them.
        let l10n = crate::localizations::of();
        let first = l10n.first_day_of_week_index() % 7;
        let initials = l10n.narrow_weekdays();
        let mut weekdays = GridView::new(7).gap(2.0);
        for column in 0..7 {
            weekdays = weekdays.cell(WeekdayCell {
                label: initials[(first + column) % 7].to_string(),
            });
        }

        // The day grid (empty cells before the 1st).
        let lead = lead_cells(year, month, first);
        let total = days_in_month(year, month);
        let mut grid = GridView::new(7).gap(2.0);
        for _ in 0..lead {
            grid = grid.cell(Day::<Msg> {
                day: 0,
                mark: DayMark::Off,
                disabled: false,
                message: None,
            });
        }
        for day in 1..=total {
            let on = enabled(day);
            grid = grid.cell(Day {
                day,
                mark: mark_of(day),
                disabled: !on,
                message: if on { Some(on_select(day)) } else { None },
            });
        }

        Self {
            children: vec![Box::new(header), Box::new(weekdays), Box::new(grid)],
            dual: false,
            day_text_style: None,
            weekday_text_style: None,
        }
    }
}

/// The floor of the weekday header's height.
const WEEKDAY_H: f32 = 22.0;

/// A weekday header cell (not clickable).
struct WeekdayCell {
    label: String,
}

impl WeekdayCell {
    fn sizing(&self, theme: Option<&Theme>) -> Style {
        // The floor, or the line the initials actually need.
        Style {
            height: Dimension::Length(frus_text::line_box(WEEKDAY_H, &weekday_style(theme), 0.0)),
            ..Default::default()
        }
    }
}

impl<Msg> Widget<Msg> for WeekdayCell {
    fn style(&self) -> Style {
        self.sizing(None)
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        self.sizing(Some(theme))
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &[]
    }

    fn paint(&self, bounds: Rect, status: Status, theme: &Theme, scene: &mut Scene) {
        let style = weekday_style(Some(theme));
        let w = frus_text::measure_resolved(&self.label, &style).width;
        scene.text(
            Point::new(bounds.x + (bounds.width - w) * 0.5, bounds.y),
            self.label.clone(),
            &style,
            theme.muted.fade(status.opacity),
        );
    }

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

impl<Msg> DatePicker<Msg> {
    fn sizing(&self, theme: Option<&Theme>) -> Style {
        // Seven cells and their gaps — the cells' **own** side, not the constant: a
        // calendar whose cells grew with the reader while its box did not would clip its
        // last column.
        let month_w = 7.0 * (cell(theme) + 2.0);
        let width = if self.dual {
            2.0 * month_w + DUAL_GAP
        } else {
            month_w
        };
        Style {
            width: Dimension::Length(width),
            flex_direction: FlexDirection::Column,
            gap: 8.0,
            ..Default::default()
        }
    }
}

impl<Msg: Clone> Widget<Msg> for DatePicker<Msg> {
    fn style(&self) -> Style {
        self.sizing(None)
    }

    fn style_themed(&self, theme: &Theme) -> Style {
        self.sizing(Some(theme))
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }

    /// Carries what this calendar was told down to its cells, over what the subtree
    /// inherited. `None` when it was told nothing, so a calendar that says nothing costs
    /// nothing and the theme's own answer stands.
    fn theme_override(&self, inherited: &Theme) -> Option<Box<Theme>> {
        if self.day_text_style.is_none() && self.weekday_text_style.is_none() {
            return None;
        }
        let mut theme = inherited.clone();
        if let Some(style) = self.day_text_style {
            theme.widgets.date_picker.day_text_style = Some(style);
        }
        if let Some(style) = self.weekday_text_style {
            theme.widgets.date_picker.weekday_text_style = Some(style);
        }
        Some(Box::new(theme))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Pick(u32),
        Nav(i32),
    }

    /// A table whose weeks start on Monday, and whose columns say so.
    struct Fr;

    impl crate::Localizations for Fr {
        fn first_day_of_week_index(&self) -> usize {
            1
        }

        fn narrow_weekdays(&self) -> [&str; 7] {
            ["D", "L", "M", "M", "J", "V", "S"]
        }

        fn months(&self) -> [&str; 12] {
            [
                "janvier",
                "f\u{e9}vrier",
                "mars",
                "avril",
                "mai",
                "juin",
                "juillet",
                "ao\u{fb}t",
                "septembre",
                "octobre",
                "novembre",
                "d\u{e9}cembre",
            ]
        }
    }

    /// **A calendar starts its week where the reader's week starts**
    /// (`calendar_date_picker.dart:1100`, `date.dart:356`).
    ///
    /// This is not a translation: it is which column a day sits in. The grid always began
    /// on Sunday, so in every country whose week starts on Monday — most of Europe — every
    /// date in the month was one column out.
    #[test]
    fn a_week_starts_where_the_reader_s_week_starts() {
        // 1 July 2026 is a Wednesday. Sunday-first: three blanks (Sun, Mon, Tue).
        assert_eq!(lead_cells(2026, 7, 0), 3);
        // Monday-first: two (Mon, Tue).
        assert_eq!(lead_cells(2026, 7, 1), 2);
        // And 1 March 2026 is a Sunday: no blanks at all Sunday-first, six Monday-first.
        assert_eq!(first_weekday(2026, 3), 0);
        assert_eq!(lead_cells(2026, 3, 0), 0);
        assert_eq!(
            lead_cells(2026, 3, 1),
            6,
            "the difference goes negative here, which is why it is taken Euclidean"
        );

        // And the grid really has that many cells.
        let cells = |l10n: Option<std::rc::Rc<dyn crate::Localizations>>| {
            let build = || {
                let dp = DatePicker::new(2026, 7, Some(11), Msg::Pick, Msg::Nav);
                Widget::<Msg>::children(&dp)[2].children().len()
            };
            match l10n {
                Some(table) => crate::localizations::scope(table, build),
                None => build(),
            }
        };
        assert_eq!(cells(None), 3 + 31, "Sunday-first");
        assert_eq!(cells(Some(std::rc::Rc::new(Fr))), 2 + 31, "Monday-first");
    }

    /// And the **column headings rotate with it**, out of a list that is always written
    /// Sunday-first (`date.dart:353`) — which is what lets one index serve both.
    #[test]
    fn the_column_headings_rotate_and_the_month_is_named() {
        let heading = |i: usize, table: Option<std::rc::Rc<dyn crate::Localizations>>| {
            let build = || {
                let dp = DatePicker::new(2026, 7, None, Msg::Pick, Msg::Nav);
                let row = &Widget::<Msg>::children(&dp)[1];
                let mut scene = Scene::new();
                row.children()[i].paint(
                    Rect::new(0.0, 0.0, 32.0, 32.0),
                    Status::default(),
                    &Theme::dark(),
                    &mut scene,
                );
                scene.primitives().iter().find_map(|p| match p {
                    frus_core::Primitive::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
            };
            match table {
                Some(table) => crate::localizations::scope(table, build),
                None => build(),
            }
        };
        assert_eq!(heading(0, None).as_deref(), Some("S"), "Sunday first");
        assert_eq!(
            heading(0, Some(std::rc::Rc::new(Fr))).as_deref(),
            Some("L"),
            "Monday first, and in the reader's letters"
        );
        assert_eq!(
            heading(6, Some(std::rc::Rc::new(Fr))).as_deref(),
            Some("D"),
            "and Sunday has moved to the end"
        );
    }

    #[test]
    fn date_math_is_correct() {
        assert_eq!(days_in_month(2024, 2), 29); // bissextile
        assert_eq!(days_in_month(2023, 2), 28);
        assert_eq!(days_in_month(2024, 4), 30);
        // 1er janvier 2024 = lundi (1).
        assert_eq!(first_weekday(2024, 1), 1);
        // 1er juillet 2026 = mercredi (3).
        assert_eq!(first_weekday(2026, 7), 3);
    }

    #[test]
    fn builds_header_weekdays_and_grid() {
        let dp = DatePicker::new(2026, 7, Some(11), Msg::Pick, Msg::Nav);
        // [header, weekdays, grid].
        assert_eq!(Widget::<Msg>::children(&dp).len(), 3);
        // The grid = filler cells + 31 days; July 2026 starts on a Wednesday
        // (3 cases vides) → 3 + 31 = 34 cellules.
        let grid = &Widget::<Msg>::children(&dp)[2];
        assert_eq!(grid.children().len(), 34);
    }

    #[test]
    fn bounded_disables_days_outside_the_range() {
        // The window [10, 20] July 2026: outside = not clickable, inside = clickable.
        let dp = DatePicker::bounded(
            2026,
            7,
            None,
            Some((2026, 7, 10)),
            Some((2026, 7, 20)),
            Msg::Pick,
            Msg::Nav,
        );
        let grid = &Widget::<Msg>::children(&dp)[2];
        // July 2026 starts on a Wednesday (3 empty cells); day `d` sits at index 3 + (d - 1).
        let at = |d: u32| grid.children()[3 + (d - 1) as usize].on_click();
        assert_eq!(at(9), None, "9 July disabled (before min)");
        assert_eq!(
            at(10),
            Some(Msg::Pick(10)),
            "10 juillet cliquable (borne min incluse)"
        );
        assert_eq!(at(15), Some(Msg::Pick(15)), "15 July clickable (inside)");
        assert_eq!(
            at(20),
            Some(Msg::Pick(20)),
            "20 July clickable (max bound included)"
        );
        assert_eq!(at(21), None, "21 July disabled (after max)");
        // Without a max bound: everything >= min is clickable.
        let open = DatePicker::bounded(
            2026,
            7,
            None,
            Some((2026, 7, 10)),
            None,
            Msg::Pick,
            Msg::Nav,
        );
        let g2 = &Widget::<Msg>::children(&open)[2];
        assert_eq!(
            g2.children()[3 + 30].on_click(),
            Some(Msg::Pick(31)),
            "31 juillet cliquable (pas de max)"
        );
    }

    #[test]
    fn filtered_disables_days_by_predicate() {
        // Blackout: 12 and 18 July unavailable; everything else clickable.
        let blackout = [(2026, 7, 12), (2026, 7, 18)];
        let dp = DatePicker::filtered(
            2026,
            7,
            None,
            move |date| !blackout.contains(&date),
            Msg::Pick,
            Msg::Nav,
        );
        let grid = &Widget::<Msg>::children(&dp)[2];
        let at = |d: u32| grid.children()[3 + (d - 1) as usize].on_click();
        assert_eq!(at(12), None, "12 juillet en blackout");
        assert_eq!(at(18), None, "18 juillet en blackout");
        assert_eq!(at(13), Some(Msg::Pick(13)), "13 juillet cliquable");
        assert_eq!(at(1), Some(Msg::Pick(1)), "1er juillet cliquable");
    }

    #[test]
    fn range_bounded_disables_days_outside_the_window() {
        // The range 10..15 is selected, but the allowed input window is [8, 20] July 2026.
        let dp = DatePicker::range_bounded(
            2026,
            7,
            Some((2026, 7, 10)),
            Some((2026, 7, 15)),
            Some((2026, 7, 8)),
            Some((2026, 7, 20)),
            Msg::Pick,
            Msg::Nav,
        );
        let grid = &Widget::<Msg>::children(&dp)[2];
        let at = |d: u32| grid.children()[3 + (d - 1) as usize].on_click();
        assert_eq!(at(7), None, "7 July outside the window (before min)");
        assert_eq!(at(8), Some(Msg::Pick(8)), "8 July clickable (min bound)");
        assert_eq!(
            at(12),
            Some(Msg::Pick(12)),
            "12 July clickable (inside the range)"
        );
        assert_eq!(at(20), Some(Msg::Pick(20)), "20 July clickable (max bound)");
        assert_eq!(at(21), None, "21 July outside the window (after max)");
    }

    #[test]
    fn range_marks_endpoints_and_interior() {
        let start = Some((2026, 7, 10));
        let end = Some((2026, 7, 15));
        assert_eq!(range_mark((2026, 7, 10), start, end), DayMark::Start);
        assert_eq!(range_mark((2026, 7, 15), start, end), DayMark::End);
        assert_eq!(range_mark((2026, 7, 12), start, end), DayMark::Between);
        assert_eq!(
            range_mark((2026, 7, 9), start, end),
            DayMark::Off,
            "before the start"
        );
        assert_eq!(
            range_mark((2026, 7, 16), start, end),
            DayMark::Off,
            "after the end"
        );
        // It crosses month boundaries: June is "between" June 15 and August 1.
        let cross = (Some((2026, 6, 15)), Some((2026, 8, 1)));
        assert_eq!(
            range_mark((2026, 7, 20), cross.0, cross.1),
            DayMark::Between
        );
        assert_eq!(range_mark((2026, 5, 31), cross.0, cross.1), DayMark::Off);
        // A selection in progress (a single bound): only the start is marked.
        assert_eq!(range_mark((2026, 7, 10), start, None), DayMark::Start);
        assert_eq!(
            range_mark((2026, 7, 12), start, None),
            DayMark::Off,
            "no open-ended band"
        );
    }

    #[test]
    fn range_builds_grid_with_clickable_days() {
        let dp = DatePicker::range(
            2026,
            7,
            Some((2026, 7, 10)),
            Some((2026, 7, 15)),
            Msg::Pick,
            Msg::Nav,
        );
        assert_eq!(Widget::<Msg>::children(&dp).len(), 3);
        let grid = &Widget::<Msg>::children(&dp)[2];
        // Leading blanks (Wednesday = 3) plus 31 days.
        assert_eq!(grid.children().len(), 34);
        // 10 July (3 blanks + days 1..10 → index 12) is still clickable.
        assert_eq!(grid.children()[12].on_click(), Some(Msg::Pick(10)));
    }

    #[test]
    fn range_dual_shows_two_consecutive_months() {
        #[derive(Clone, Debug, PartialEq)]
        enum M {
            Pick(i32, u32, u32),
            Nav(i32),
        }
        // December → January of the following year (a year rollover).
        let dp = DatePicker::range_dual(
            2026,
            12,
            Some((2026, 12, 28)),
            Some((2027, 1, 3)),
            |(y, m, d)| M::Pick(y, m, d),
            M::Nav,
        );
        // A single child: the row of two months.
        let row = &Widget::<M>::children(&dp);
        assert_eq!(row.len(), 1);
        let months = row[0].children();
        assert_eq!(months.len(), 2, "two calendars side by side");
        // The right-hand month is January 2027: clicking its 3rd reports the complete date.
        let jan_grid = &months[1].children()[2];
        // January 2027 starts on a Friday (5 empty cells) → the 3rd is at index 5 + 2 = 7.
        assert_eq!(first_weekday(2027, 1), 5);
        assert_eq!(jan_grid.children()[7].on_click(), Some(M::Pick(2027, 1, 3)));
    }
}
