//! `DatePicker` — a month calendar. It is controlled: the application keeps the month on
//! show and the selection; the calendar reports clicked days and month changes. It can
//! disable days, and select a range.

use frus::{DatePicker, State, StateContext, StatefulWidget, Text, Widget};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(DatePickerDemo.into_widget())
}

/// A date as the pickers speak it: `(year, month 1–12, day)`.
pub type Date = (i32, u32, u32);

/// The month `delta` months away from `(year, month)`. Plain arithmetic, tested in `tests/date_picker_test.rs`.
pub fn shift_month(year: i32, month: u32, delta: i32) -> (i32, u32) {
    let index = year * 12 + month as i32 - 1 + delta;
    (index.div_euclid(12), index.rem_euclid(12) as u32 + 1)
}

/// Day of the week, 0 = Sunday (Sakamoto's method), to disable weekends.
pub fn weekday((year, month, day): Date) -> u32 {
    const T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    ((y + y / 4 - y / 100 + y / 400 + T[month as usize - 1] + day as i32).rem_euclid(7)) as u32
}

pub struct DatePickerDemo;

pub struct DatePickerDemoState {
    /// The month each calendar shows.
    shown: (i32, u32),
    selected: Option<Date>,
    weekday_shown: (i32, u32),
    weekday: Option<Date>,
    range_shown: (i32, u32),
    pub range: (Option<Date>, Option<Date>),
}

impl DatePickerDemoState {
    /// The usual range rule: the first click starts a range, the second ends it (or
    /// starts again if it is before the start).
    pub fn pick_range_day(&mut self, date: Date) {
        self.range = match self.range {
            (Some(start), None) if date >= start => (Some(start), Some(date)),
            _ => (Some(date), None),
        };
    }
}

impl StatefulWidget for DatePickerDemo {
    type State = DatePickerDemoState;

    fn create_state(&self) -> DatePickerDemoState {
        DatePickerDemoState {
            shown: (2026, 10),
            selected: Some((2026, 10, 15)),
            weekday_shown: (2026, 10),
            weekday: None,
            range_shown: (2026, 10),
            range: (Some((2026, 10, 12)), Some((2026, 10, 18))),
        }
    }
}

pub fn describe(date: Option<Date>) -> String {
    match date {
        Some((y, m, d)) => format!("{y}-{m:02}-{d:02}"),
        None => "—".to_string(),
    }
}

impl State for DatePickerDemoState {
    type Widget = DatePickerDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let (year, month) = self.shown;
        // The selected day only shows in the month it belongs to.
        let selected = self
            .selected
            .filter(|(y, m, _)| (*y, *m) == self.shown)
            .map(|(_, _, d)| d);
        let simple = DatePicker::new(
            year,
            month,
            selected,
            // A day of the month on show was clicked.
            cx.handler(|s: &mut DatePickerDemoState, day: u32| {
                s.selected = Some((s.shown.0, s.shown.1, day))
            }),
            // The arrows: -1 or +1 month.
            cx.handler(|s: &mut DatePickerDemoState, delta: i32| {
                s.shown = shift_month(s.shown.0, s.shown.1, delta)
            }),
        );

        // `filtered` disables any day the predicate refuses: here, weekends.
        let (wy, wm) = self.weekday_shown;
        let weekdays = DatePicker::filtered(
            wy,
            wm,
            self.weekday
                .filter(|(y, m, _)| (*y, *m) == self.weekday_shown)
                .map(|(_, _, d)| d),
            |date| !matches!(weekday(date), 0 | 6),
            cx.handler(|s: &mut DatePickerDemoState, day: u32| {
                s.weekday = Some((s.weekday_shown.0, s.weekday_shown.1, day))
            }),
            cx.handler(|s: &mut DatePickerDemoState, delta: i32| {
                s.weekday_shown = shift_month(s.weekday_shown.0, s.weekday_shown.1, delta)
            }),
        );

        // `range` highlights an interval; the application decides which end a click sets.
        let (ry, rm) = self.range_shown;
        let range = DatePicker::range(
            ry,
            rm,
            self.range.0,
            self.range.1,
            cx.handler(|s: &mut DatePickerDemoState, day: u32| {
                let (y, m) = s.range_shown;
                s.pick_range_day((y, m, day))
            }),
            cx.handler(|s: &mut DatePickerDemoState, delta: i32| {
                s.range_shown = shift_month(s.range_shown.0, s.range_shown.1, delta)
            }),
        );

        sections(vec![
            Box::new(section("A single day", simple)),
            Box::new(Text::new(format!("Selected: {}", describe(self.selected)))),
            Box::new(section("Weekdays only", weekdays)),
            Box::new(Text::new(format!("Meeting on: {}", describe(self.weekday)))),
            Box::new(section("A range", range)),
            Box::new(Text::new(format!(
                "From {} to {}",
                describe(self.range.0),
                describe(self.range.1)
            ))),
        ])
    }
}
