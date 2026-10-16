//! `SegmentedButton` — one outlined control with a few segments, exactly one of them
//! chosen. For switching between views or modes (day / week / month).

use frus::{Color, SegmentedButton, State, StateContext, StatefulWidget, Text, Widget};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(SegmentedDemo.into_widget())
}

const PERIODS: [&str; 3] = ["Day", "Week", "Month"];

struct SegmentedDemo;

struct SegmentedDemoState {
    period: usize,
    size: usize,
}

impl StatefulWidget for SegmentedDemo {
    type State = SegmentedDemoState;

    fn create_state(&self) -> SegmentedDemoState {
        SegmentedDemoState { period: 1, size: 0 }
    }
}

impl State for SegmentedDemoState {
    type Widget = SegmentedDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        // The control reports the index that was picked; `cx.handler` stores it.
        let periods = PERIODS.iter().fold(
            SegmentedButton::new(
                self.period,
                cx.handler(|s: &mut SegmentedDemoState, i: usize| s.period = i),
            ),
            |picker, label| picker.segment(*label),
        );

        sections(vec![
            Box::new(section("Single choice", periods)),
            Box::new(Text::new(format!(
                "Showing one {}",
                PERIODS[self.period].to_lowercase()
            ))),
            // Every colour and measurement can be overridden; `show_selected_icon(false)`
            // drops the checkmark when the fill alone is clear enough.
            Box::new(section(
                "Custom look",
                SegmentedButton::new(
                    self.size,
                    cx.handler(|s: &mut SegmentedDemoState, i: usize| s.size = i),
                )
                .segment("S")
                .segment("M")
                .segment("L")
                .segment("XL")
                .selected_color(Color::hex("#FDE68A"))
                .selected_label_color(Color::hex("#78350F"))
                .border_color(Color::hex("#D97706"))
                .radius(8.0)
                .show_selected_icon(false),
            )),
            Box::new(section(
                "Disabled",
                SegmentedButton::new(0, cx.handler(|_: &mut SegmentedDemoState, _: usize| {}))
                    .segment("On")
                    .segment("Off")
                    .enabled(false),
            )),
        ])
    }
}
