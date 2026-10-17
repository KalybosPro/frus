//! `Slider` — a value picked by dragging along a track: continuous, or in steps with
//! `divisions`, with a bubble showing the value while dragging.

use frus::{
    Color, Insets, Padding, ShowValueIndicator, Slider, State, StateContext, StatefulWidget, Text,
    Widget,
};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(SliderDemo.into_widget())
}

struct SliderDemo;

struct SliderDemoState {
    volume: f32,
    rating: f32,
    temperature: f32,
    /// Changed only when a drag **ends**: for work too costly to do on every pixel.
    committed: f32,
}

impl StatefulWidget for SliderDemo {
    type State = SliderDemoState;

    fn create_state(&self) -> SliderDemoState {
        SliderDemoState {
            volume: 0.5,
            rating: 3.0,
            temperature: 21.0,
            committed: 21.0,
        }
    }
}

impl State for SliderDemoState {
    type Widget = SliderDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        sections(vec![
            // From 0 to 1 unless told otherwise.
            Box::new(section(
                "Continuous",
                Slider::new(self.volume)
                    .width(300.0)
                    .on_change(cx.handler(|s: &mut SliderDemoState, v: f32| s.volume = v)),
            )),
            Box::new(Text::new(format!("Volume: {:.0}%", self.volume * 100.0))),
            // `divisions` snaps to steps; `value_label` writes the bubble, shown here all
            // the time (by default only while dragging). The padding is the bubble's room.
            Box::new(section(
                "Steps with a value label",
                Padding::new(
                    Insets::new(28.0, 0.0, 0.0, 0.0),
                    Slider::new(self.rating)
                        .range(1.0, 5.0)
                        .divisions(4)
                        .value_label(|v| format!("{v:.0} stars"))
                        .show_value_indicator(ShowValueIndicator::AlwaysVisible)
                        .width(300.0)
                        .on_change(cx.handler(|s: &mut SliderDemoState, v: f32| s.rating = v)),
                ),
            )),
            // `on_change_end` fires once, on release.
            Box::new(section(
                "Custom range, colours and change end",
                Slider::new(self.temperature)
                    .range(10.0, 30.0)
                    .divisions(40)
                    .value_label(|v| format!("{v:.1} °C"))
                    .active_color(Color::hex("#DC2626"))
                    .inactive_color(Color::hex("#FECACA"))
                    .thumb_color(Color::hex("#991B1B"))
                    .width(300.0)
                    .on_change(cx.handler(|s: &mut SliderDemoState, v: f32| s.temperature = v))
                    .on_change_end(cx.handler(|s: &mut SliderDemoState, v: f32| s.committed = v)),
            )),
            Box::new(Text::new(format!(
                "Dragging: {:.1} °C · saved: {:.1} °C",
                self.temperature, self.committed
            ))),
            // A second, lighter track: how much of a video is buffered, say.
            Box::new(section(
                "Secondary track, disabled",
                Slider::new(0.3)
                    .secondary_track_value(0.7)
                    .width(300.0)
                    .enabled(false),
            )),
        ])
    }
}
