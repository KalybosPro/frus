use std::time::Duration;

use frus::{
    Align, AppBar, Button, Center, Column, FloatingActionButton, IconButton, Icons, Justify, Row,
    Scaffold, SegmentedButton, State, StateContext, StatefulWidget, StatelessWidget, Text, Variant,
    Widget,
};

use crate::features::counter::{
    states::counterstate::CounterState, widgets::count_display::CountDisplay,
};

/// The page's **configuration**: what its parent decides. Here, the step it starts with.
///
/// The widget is made again on every rebuild; what must survive a rebuild lives in its
/// `State` (`CounterState`), which the framework creates once and keeps.
#[derive(Clone)]
pub(crate) struct CounterPage {
    initial_step: i64,
}

impl CounterPage {
    pub(crate) fn new(initial_step: i64) -> Self {
        Self { initial_step }
    }
}

impl StatefulWidget for CounterPage {
    type State = CounterState;

    /// Called once, when the page first appears.
    fn create_state(&self) -> CounterState {
        CounterState::new(self.initial_step)
    }
}

/// The steps the segmented button offers.
const STEPS: [i64; 3] = [1, 5, 10];

impl State for CounterState {
    type Widget = CounterPage;

    /// Describes the page for the current state.
    ///
    /// Every handler is made with `cx.callback(..)`: it changes the state through one of
    /// its methods, then the framework rebuilds. `build` itself never changes the state.
    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        // A timer is asked for by the build that wants it, and stops as soon as a build
        // stops asking: switch automatic mode off and there is no timer left.
        if self.auto {
            cx.use_interval(Duration::from_secs(1), cx.callback(CounterState::increment));
        }

        // `cx.handler` is `cx.callback` for a widget that hands back a value — here, the
        // index of the segment that was picked.
        let selected = STEPS.iter().position(|&s| s == self.step).unwrap_or(0);
        let step_picker = STEPS.iter().fold(
            SegmentedButton::new(
                selected,
                cx.handler(|state: &mut CounterState, index: usize| state.set_step(STEPS[index])),
            ),
            |picker, step| picker.segment(format!("±{step}")),
        );

        let controls = Row::new()
            .spacing(16.0)
            .main_axis_alignment(Justify::Center)
            .child(
                Button::new("−")
                    .variant(Variant::Outlined)
                    .on_press(cx.callback(CounterState::decrement)),
            )
            .child(
                Button::new(if self.auto { "Stop auto" } else { "Start auto" })
                    .variant(Variant::Tonal)
                    .on_press(cx.callback(CounterState::toggle_auto)),
            );

        let body = Column::new()
            .spacing(32.0)
            .main_axis_alignment(Justify::Center)
            .cross_axis_alignment(Align::Center)
            .child(CountDisplay::new(self.count).into_widget())
            .child(step_picker)
            .child(controls);

        let app_bar = AppBar::new()
            .title(Text::new("Counter"))
            .action(
                IconButton::new(Icons::RESTART_ALT)
                    .label("Reset")
                    .on_press(cx.callback(CounterState::reset)),
            )
            .build();

        Scaffold::new()
            .background(cx.theme().background)
            .app_bar(app_bar)
            .body(Center::new(body))
            .fab(
                FloatingActionButton::new(Icons::ADD)
                    .on_press(cx.callback(CounterState::increment)),
            )
            .build()
    }
}
