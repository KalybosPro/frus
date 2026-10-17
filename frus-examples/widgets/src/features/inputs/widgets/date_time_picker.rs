//! `DateTimePicker` — a calendar above a time picker, with a summary of the choice on
//! top. For one moment: an appointment, a reminder.

use frus::{DateTimePicker, State, StateContext, StatefulWidget, Widget};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(DateTimeDemo.into_widget())
}

/// The month `delta` months away from `(year, month)`.
fn shift_month(year: i32, month: u32, delta: i32) -> (i32, u32) {
    let index = year * 12 + month as i32 - 1 + delta;
    (index.div_euclid(12), index.rem_euclid(12) as u32 + 1)
}

struct DateTimeDemo;

struct DateTimeDemoState {
    year: i32,
    month: u32,
    day: Option<u32>,
    hour: u32,
    minute: u32,
}

impl StatefulWidget for DateTimeDemo {
    type State = DateTimeDemoState;

    fn create_state(&self) -> DateTimeDemoState {
        DateTimeDemoState {
            year: 2026,
            month: 10,
            day: Some(20),
            hour: 10,
            minute: 30,
        }
    }
}

impl State for DateTimeDemoState {
    type Widget = DateTimeDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        // Four handlers — day, month arrows, hour, minute — each storing one field. The
        // summary at the top appears once a day is chosen.
        let picker = DateTimePicker::new(
            self.year,
            self.month,
            self.day,
            self.hour,
            self.minute,
            cx.handler(|s: &mut DateTimeDemoState, day: u32| s.day = Some(day)),
            cx.handler(|s: &mut DateTimeDemoState, delta: i32| {
                (s.year, s.month) = shift_month(s.year, s.month, delta);
                // The day belonged to the other month.
                s.day = None;
            }),
            cx.handler(|s: &mut DateTimeDemoState, hour: u32| s.hour = hour),
            cx.handler(|s: &mut DateTimeDemoState, minute: u32| s.minute = minute),
        );

        sections(vec![Box::new(section("Pick a day, then a time", picker))])
    }
}
