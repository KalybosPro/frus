//! [`DateTimeRange`]: a **date-and-time range** in one widget — the dual calendar of
//! [`crate::DatePicker::range_dual`] above the [`crate::TimeRange`] time range, topped by
//! a **"start → end" summary**. For booking a complete slot: "from 28 July 09:00 to 3
//! August 17:30".
//!
//! Purely composite and **controlled**: it combines the two sub-pickers and relays their
//! messages; the state, dates and times, lives in the application, which decides which
//! endpoint a clicked day lands on, as the date range does.

use frus_core::{Rect, Scene};
use frus_layout::{Dimension, FlexDirection, Style};

use crate::datepicker::DatePicker;
use crate::interaction::Status;
use crate::text::Text;
use crate::theme::Theme;
use crate::timepicker::{Endpoint, TimeField, TimeRange};
use crate::widget::Widget;

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// A date-and-time range picker.
pub struct DateTimeRange<Msg> {
    children: Vec<Box<dyn Widget<Msg>>>,
}

impl<Msg: Clone + 'static> DateTimeRange<Msg> {
    /// Creates the picker. The **date** part shows `year`/`month` (1–12) and the month
    /// after, highlighting the `[start_date, end_date]` range — `on_date((year, month,
    /// day))` on click, `on_nav(±1)` to shift the pair — while the **time** part shows
    /// `start_time`/`end_time` (`on_time(endpoint, field, value)`). A "start → end"
    /// summary tops it all once **both** dates are set.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        year: i32,
        month: u32,
        start_date: Option<(i32, u32, u32)>,
        end_date: Option<(i32, u32, u32)>,
        start_time: (u32, u32),
        end_time: (u32, u32),
        on_date: impl Fn((i32, u32, u32)) -> Msg + 'static,
        on_nav: impl Fn(i32) -> Msg + 'static,
        on_time: impl Fn(Endpoint, TimeField, u32) -> Msg + 'static,
    ) -> Self {
        let calendar = DatePicker::range_dual(year, month, start_date, end_date, on_date, on_nav);
        let times = TimeRange::new(start_time, end_time, on_time);

        let mut children: Vec<Box<dyn Widget<Msg>>> = Vec::new();
        if let (Some(s), Some(e)) = (start_date, end_date) {
            let stamp = |d: (i32, u32, u32), t: (u32, u32)| {
                format!(
                    "{} {}, {}  {:02}:{:02}",
                    MONTHS[(d.1 - 1) as usize],
                    d.2,
                    d.0,
                    t.0,
                    t.1
                )
            };
            let summary = format!("{}  →  {}", stamp(s, start_time), stamp(e, end_time));
            children.push(Box::new(Text::new(summary).size(16.0)));
        }
        children.push(Box::new(calendar));
        children.push(Box::new(times));

        Self { children }
    }
}

impl<Msg: Clone> Widget<Msg> for DateTimeRange<Msg> {
    fn style(&self) -> Style {
        Style {
            width: Dimension::Auto,
            flex_direction: FlexDirection::Column,
            gap: 16.0,
            ..Default::default()
        }
    }

    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        &self.children
    }

    fn paint(&self, _bounds: Rect, _status: Status, _theme: &Theme, _scene: &mut Scene) {}

    fn on_click(&self) -> Option<Msg> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_ui, Runtime, Size};
    use frus_core::Primitive;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Date(i32, u32, u32),
        Nav(i32),
        Time(Endpoint, TimeField, u32),
    }

    fn build(start: Option<(i32, u32, u32)>, end: Option<(i32, u32, u32)>) -> DateTimeRange<Msg> {
        DateTimeRange::new(
            2026,
            7,
            start,
            end,
            (9, 0),
            (17, 30),
            |(y, m, d)| Msg::Date(y, m, d),
            Msg::Nav,
            Msg::Time,
        )
    }

    #[test]
    fn summary_appears_only_with_both_dates() {
        // Neither or one endpoint gives [calendar, times]; both give [summary, calendar, times].
        assert_eq!(Widget::<Msg>::children(&build(None, None)).len(), 2);
        assert_eq!(
            Widget::<Msg>::children(&build(Some((2026, 7, 28)), None)).len(),
            2
        );
        assert_eq!(
            Widget::<Msg>::children(&build(Some((2026, 7, 28)), Some((2026, 8, 3)))).len(),
            3,
        );
    }

    #[test]
    fn renders_the_combined_summary() {
        let dtr = build(Some((2026, 7, 28)), Some((2026, 8, 3)));
        let ui = build_ui(
            &dtr,
            Size::new(560.0, 700.0),
            &Runtime::default(),
            &Theme::default(),
        );
        let has = ui.scene().primitives().iter().any(|p| {
            matches!(p, Primitive::Text { text, .. }
                if text == "July 28, 2026  09:00  →  August 3, 2026  17:30")
        });
        assert!(has, "the start → end summary is shown");
    }
}
