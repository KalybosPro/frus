//! `DateTimeRange` — two months side by side for a date range, a start and an end time
//! below, and a "start → end" summary on top. For booking a whole slot.

use frus::{
    Axis, Callback, DateTimeRange, Endpoint, SingleChildScrollView, State, StateContext,
    StatefulWidget, TimeField, Widget,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(BookingDemo.into_widget())
}

type Date = (i32, u32, u32);
type Time = (u32, u32);

/// The month `delta` months away from `(year, month)`.
fn shift_month(year: i32, month: u32, delta: i32) -> (i32, u32) {
    let index = year * 12 + month as i32 - 1 + delta;
    (index.div_euclid(12), index.rem_euclid(12) as u32 + 1)
}

struct BookingDemo;

struct BookingState {
    /// The first of the two months on show.
    shown: (i32, u32),
    start: Option<Date>,
    end: Option<Date>,
    start_time: Time,
    end_time: Time,
}

impl BookingState {
    /// First click: the start. Second click on or after it: the end. Anything else
    /// starts again.
    fn pick(&mut self, date: Date) {
        match (self.start, self.end) {
            (Some(start), None) if date >= start => self.end = Some(date),
            _ => {
                self.start = Some(date);
                self.end = None;
            }
        }
    }

    fn set_time(&mut self, endpoint: Endpoint, field: TimeField, value: u32) {
        let time = match endpoint {
            Endpoint::Start => &mut self.start_time,
            Endpoint::End => &mut self.end_time,
        };
        match field {
            TimeField::Hour => time.0 = value,
            TimeField::Minute => time.1 = value,
        }
    }
}

impl StatefulWidget for BookingDemo {
    type State = BookingState;

    fn create_state(&self) -> BookingState {
        BookingState {
            shown: (2026, 10),
            start: Some((2026, 10, 28)),
            end: Some((2026, 11, 3)),
            start_time: (9, 0),
            end_time: (17, 30),
        }
    }
}

impl State for BookingState {
    type Widget = BookingDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        // The calendar reports **whole dates**, since a click can land in either month.
        let picker = DateTimeRange::new(
            self.shown.0,
            self.shown.1,
            self.start,
            self.end,
            self.start_time,
            self.end_time,
            cx.handler(|s: &mut BookingState, date: Date| s.pick(date)),
            cx.handler(|s: &mut BookingState, delta: i32| {
                s.shown = shift_month(s.shown.0, s.shown.1, delta)
            }),
            {
                // Three values per change, so no `cx.handler`: the closure makes the
                // `Callback` itself, through a handle to this state.
                let handle = cx.handle();
                move |endpoint, field, value| {
                    let handle = handle.clone();
                    Callback::new(move || handle.set_state(|s| s.set_time(endpoint, field, value)))
                }
            },
        );

        // Two months side by side are wider than a phone: let them scroll sideways
        // rather than run off the edge.
        sections(vec![Box::new(section(
            "Book a stay",
            SingleChildScrollView::new()
                .axis(Axis::Horizontal)
                .child(picker),
        ))])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_clicks_make_a_range_and_a_third_starts_again() {
        let mut state = BookingDemo.create_state();
        state.start = None;
        state.end = None;
        state.pick((2026, 10, 30));
        state.pick((2026, 11, 2));
        assert_eq!(
            (state.start, state.end),
            (Some((2026, 10, 30)), Some((2026, 11, 2)))
        );
        state.pick((2026, 10, 1));
        assert_eq!((state.start, state.end), (Some((2026, 10, 1)), None));
    }
}
