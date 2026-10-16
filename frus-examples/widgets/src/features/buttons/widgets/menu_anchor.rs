//! `MenuAnchor` — a floating panel with **any content**, anchored to a widget and closed
//! by a click outside. A popup menu is a list of rows; this is for everything else.

use frus::{
    Align, Button, Column, FontWeight, Insets, MenuAnchor, Slider, State, StateContext,
    StatefulWidget, Text, Variant, Widget,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(AnchorDemo.into_widget())
}

struct AnchorDemo;

struct AnchorDemoState {
    open: bool,
    volume: f32,
}

impl StatefulWidget for AnchorDemo {
    type State = AnchorDemoState;

    fn create_state(&self) -> AnchorDemoState {
        AnchorDemoState {
            open: false,
            volume: 0.4,
        }
    }
}

impl State for AnchorDemoState {
    type Widget = AnchorDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        // The panel holds live widgets: dragging the slider changes the state while the
        // panel stays open.
        let panel = Column::new()
            .spacing(8.0)
            .cross_axis_alignment(Align::Start)
            .child(Text::new("Volume").weight(FontWeight::SemiBold))
            .child(
                Slider::new(self.volume)
                    .width(200.0)
                    .on_change(cx.handler(|s: &mut AnchorDemoState, v: f32| s.volume = v)),
            )
            .child(
                Button::new("Done")
                    .variant(Variant::Text)
                    .on_press(cx.callback(|s: &mut AnchorDemoState| s.open = false)),
            );

        let anchor = MenuAnchor::new(
            Button::new(format!("Volume: {:.0}%", self.volume * 100.0))
                .variant(Variant::Tonal)
                .on_press(cx.callback(|s: &mut AnchorDemoState| s.open = !s.open)),
            self.open,
            cx.callback(|s: &mut AnchorDemoState| s.open = false),
        )
        .content(panel)
        .menu_padding(Insets::uniform(16.0))
        .radius(16.0);

        sections(vec![Box::new(section("A panel with a slider", anchor))])
    }
}
