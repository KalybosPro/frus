//! `TimePicker` and `TimeRange` — a time of day picked from an hour grid and a minute
//! grid; `TimeRange` is two of them, for a start and an end. Times always come back in
//! 24-hour form, whatever the picker displays.

use frus::{
    Axis, Callback, Endpoint, SingleChildScrollView, State, StateContext, StatefulWidget, Text,
    TimeField, TimePicker, TimeRange, Widget,
};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(TimePickerDemo.into_widget())
}

pub struct TimePickerDemo;

pub struct TimePickerDemoState {
    alarm: (u32, u32),
    meeting: (u32, u32),
    pub shift: ((u32, u32), (u32, u32)),
}

impl TimePickerDemoState {
    /// `TimeRange` reports one change at a time: which end, which field, the new value.
    pub fn change_shift(&mut self, endpoint: Endpoint, field: TimeField, value: u32) {
        let time = match endpoint {
            Endpoint::Start => &mut self.shift.0,
            Endpoint::End => &mut self.shift.1,
        };
        match field {
            TimeField::Hour => time.0 = value,
            TimeField::Minute => time.1 = value,
        }
    }
}

impl StatefulWidget for TimePickerDemo {
    type State = TimePickerDemoState;

    fn create_state(&self) -> TimePickerDemoState {
        TimePickerDemoState {
            alarm: (7, 30),
            meeting: (14, 0),
            shift: ((9, 0), (17, 30)),
        }
    }
}

impl State for TimePickerDemoState {
    type Widget = TimePickerDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        sections(vec![
            // 24-hour, minutes in steps of five by default.
            Box::new(section(
                "24-hour",
                TimePicker::new(
                    self.alarm.0,
                    self.alarm.1,
                    cx.handler(|s: &mut TimePickerDemoState, h: u32| s.alarm.0 = h),
                    cx.handler(|s: &mut TimePickerDemoState, m: u32| s.alarm.1 = m),
                ),
            )),
            Box::new(Text::new(format!(
                "Alarm at {:02}:{:02}",
                self.alarm.0, self.alarm.1
            ))),
            // A 12-hour dial with AM/PM, and quarter hours only.
            Box::new(section(
                "12-hour, quarter hours",
                TimePicker::new(
                    self.meeting.0,
                    self.meeting.1,
                    cx.handler(|s: &mut TimePickerDemoState, h: u32| s.meeting.0 = h),
                    cx.handler(|s: &mut TimePickerDemoState, m: u32| s.meeting.1 = m),
                )
                .hour12()
                .minute_step(15),
            )),
            Box::new(Text::new(format!(
                "Meeting at {:02}:{:02} (stored in 24-hour time)",
                self.meeting.0, self.meeting.1
            ))),
            // A start and an end side by side — wider than a phone, so it may scroll
            // sideways.
            Box::new(section(
                "TimeRange",
                SingleChildScrollView::new()
                    .axis(Axis::Horizontal)
                    .child(TimeRange::new(self.shift.0, self.shift.1, {
                        // Three values per change, so no `cx.handler`: the closure makes the
                        // `Callback` itself, through a handle to this state.
                        let handle = cx.handle();
                        move |endpoint, field, value| {
                            let handle = handle.clone();
                            Callback::new(move || {
                                handle.set_state(|s| s.change_shift(endpoint, field, value))
                            })
                        }
                    })),
            )),
            Box::new(Text::new(format!(
                "Shift: {:02}:{:02} → {:02}:{:02}",
                self.shift.0 .0, self.shift.0 .1, self.shift.1 .0, self.shift.1 .1
            ))),
        ])
    }
}
