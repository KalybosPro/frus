//! `LinearProgressIndicator` and `CircularProgressIndicator` — how far along some work is.
//! **Determinate** when you know the share done (a download), **indeterminate** when you
//! do not (waiting for a server).

use std::time::Duration;

use frus::{
    Align, Button, CircularProgressIndicator, Color, Column, LinearProgressIndicator, Row, State,
    StateContext, StatefulWidget, Text, Variant, Widget,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(ProgressDemo.into_widget())
}

struct ProgressDemo;

struct ProgressDemoState {
    /// From 0 to 1.
    progress: f32,
    running: bool,
}

impl ProgressDemoState {
    /// One tick of the fake download: 2% more, stopping at the end.
    fn tick(&mut self) {
        self.progress = (self.progress + 0.02).min(1.0);
        if self.progress >= 1.0 {
            self.running = false;
        }
    }

    fn start(&mut self) {
        if self.progress >= 1.0 {
            self.progress = 0.0;
        }
        self.running = true;
    }
}

impl StatefulWidget for ProgressDemo {
    type State = ProgressDemoState;

    fn create_state(&self) -> ProgressDemoState {
        ProgressDemoState {
            progress: 0.35,
            running: false,
        }
    }
}

impl State for ProgressDemoState {
    type Widget = ProgressDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        // The timer exists only while a build asks for it: pause, and it stops.
        if self.running {
            cx.use_interval(
                Duration::from_millis(100),
                cx.callback(ProgressDemoState::tick),
            );
        }

        let controls = Row::new()
            .spacing(8.0)
            .child(
                Button::new(if self.running { "Pause" } else { "Start" }).on_press(cx.callback(
                    |s: &mut ProgressDemoState| {
                        if s.running {
                            s.running = false
                        } else {
                            s.start()
                        }
                    },
                )),
            )
            .child(
                Button::new("Reset")
                    .variant(Variant::Outlined)
                    .on_press(cx.callback(|s: &mut ProgressDemoState| {
                        s.progress = 0.0;
                        s.running = false;
                    })),
            );

        sections(vec![
            // `new(value)` with a value from 0 to 1.
            Box::new(section(
                "Determinate",
                Column::new()
                    .spacing(16.0)
                    .cross_axis_alignment(Align::Start)
                    .child(controls)
                    .child(LinearProgressIndicator::new(self.progress).width(300.0))
                    .child(
                        Row::new()
                            .spacing(16.0)
                            .cross_axis_alignment(Align::Center)
                            .child(CircularProgressIndicator::determinate(self.progress))
                            .child(Text::new(format!("{:.0}%", self.progress * 100.0))),
                    ),
            )),
            // No value: the bar runs and the arc spins for as long as they are shown.
            Box::new(section(
                "Indeterminate",
                Column::new()
                    .spacing(16.0)
                    .cross_axis_alignment(Align::Start)
                    .child(LinearProgressIndicator::indeterminate().width(300.0))
                    .child(CircularProgressIndicator::new()),
            )),
            // Colours, thickness, size and corners are all overridable.
            Box::new(section(
                "Custom look",
                Column::new()
                    .spacing(16.0)
                    .cross_axis_alignment(Align::Start)
                    .child(
                        LinearProgressIndicator::new(self.progress)
                            .width(300.0)
                            .min_height(10.0)
                            .radius(5.0)
                            .color(Color::hex("#16A34A"))
                            .track_color(Color::hex("#DCFCE7")),
                    )
                    .child(
                        Row::new()
                            .spacing(24.0)
                            .child(
                                CircularProgressIndicator::new()
                                    .size(24.0)
                                    .stroke_width(3.0),
                            )
                            .child(
                                CircularProgressIndicator::determinate(self.progress)
                                    .size(64.0)
                                    .stroke_width(8.0)
                                    .color(Color::hex("#DB2777"))
                                    .track_color(Color::hex("#FCE7F3")),
                            ),
                    ),
            )),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_download_stops_at_the_end() {
        let mut state = ProgressDemo.create_state();
        state.start();
        for _ in 0..100 {
            state.tick();
        }
        assert_eq!(state.progress, 1.0);
        assert!(!state.running);
        // Starting again from the end starts from zero.
        state.start();
        assert_eq!(state.progress, 0.0);
    }
}
