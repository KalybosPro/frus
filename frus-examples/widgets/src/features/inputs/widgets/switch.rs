//! `Switch` — an on/off toggle that takes effect at once (a setting), as opposed to a
//! checkbox, which is usually confirmed later (a form).

use frus::{Align, Color, Icons, Row, State, StateContext, StatefulWidget, Switch, Text, Widget};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(SwitchDemo.into_widget())
}

struct SwitchDemo;

struct SwitchDemoState {
    airplane: bool,
    dark: bool,
    custom: bool,
}

impl StatefulWidget for SwitchDemo {
    type State = SwitchDemoState;

    fn create_state(&self) -> SwitchDemoState {
        SwitchDemoState {
            airplane: false,
            dark: true,
            custom: true,
        }
    }
}

/// A switch with its label, the way a settings row shows one.
fn labelled(label: &str, switch: Switch) -> Row {
    Row::new()
        .spacing(12.0)
        .cross_axis_alignment(Align::Center)
        .child(switch)
        .child(Text::new(label))
}

impl State for SwitchDemoState {
    type Widget = SwitchDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        sections(vec![
            // `on_toggle` reports the new value.
            Box::new(section(
                "Simple",
                labelled(
                    if self.airplane {
                        "Airplane mode is on"
                    } else {
                        "Airplane mode is off"
                    },
                    Switch::new(self.airplane)
                        .on_toggle(cx.handler(|s: &mut SwitchDemoState, on: bool| s.airplane = on)),
                ),
            )),
            // An icon in the thumb makes the state readable without colour.
            Box::new(section(
                "Thumb icons",
                labelled(
                    "Dark theme",
                    Switch::new(self.dark)
                        .thumb_icon(Icons::CHECK)
                        .inactive_thumb_icon(Icons::CLOSE)
                        .on_toggle(cx.handler(|s: &mut SwitchDemoState, on: bool| s.dark = on)),
                ),
            )),
            Box::new(section(
                "Custom colours",
                labelled(
                    "Orange",
                    Switch::new(self.custom)
                        .track_color(Color::hex("#EA580C"))
                        .thumb_color(Color::hex("#FFF7ED"))
                        .inactive_track_color(Color::hex("#FED7AA"))
                        .on_toggle(cx.handler(|s: &mut SwitchDemoState, on: bool| s.custom = on)),
                ),
            )),
            Box::new(section(
                "Disabled",
                Row::new()
                    .spacing(12.0)
                    .child(Switch::new(true).enabled(false))
                    .child(Switch::new(false).enabled(false)),
            )),
        ])
    }
}
