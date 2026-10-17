//! `BackButton`, `CloseButton`, `DrawerButton` and `EndDrawerButton` — icon buttons
//! whose meaning is fixed. They know their icon and their accessible name (translated
//! by the framework), so an app bar never spells them out by hand.

use frus::{
    BackButton, CloseButton, Color, DrawerButton, EndDrawerButton, State, StateContext,
    StatefulWidget, Text, Widget, Wrap,
};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(ActionButtonsDemo.into_widget())
}

struct ActionButtonsDemo;

struct ActionButtonsDemoState {
    last: &'static str,
}

impl StatefulWidget for ActionButtonsDemo {
    type State = ActionButtonsDemoState;

    fn create_state(&self) -> ActionButtonsDemoState {
        ActionButtonsDemoState {
            last: "nothing yet",
        }
    }
}

impl State for ActionButtonsDemoState {
    type Widget = ActionButtonsDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let say =
            |what: &'static str| cx.callback(move |s: &mut ActionButtonsDemoState| s.last = what);

        sections(vec![
            // In a real page: `BackButton` pops the route (`cx.router().pop()`),
            // `CloseButton` dismisses a full-screen dialog, and the drawer buttons open
            // the scaffold's drawers. An `AppBar` adds the right one by itself.
            Box::new(section(
                "The four of them",
                Wrap::new()
                    .gap(16.0)
                    .run_gap(8.0)
                    .child(BackButton::new().on_press(say("Back")))
                    .child(CloseButton::new().on_press(say("Close")))
                    .child(DrawerButton::new().on_press(say("Open drawer")))
                    .child(EndDrawerButton::new().on_press(say("Open end drawer"))),
            )),
            Box::new(Text::new(format!("Pressed: {}", self.last))),
            Box::new(section(
                "Custom colour and size, and disabled",
                Wrap::new()
                    .gap(16.0)
                    .run_gap(8.0)
                    .child(
                        BackButton::new()
                            .color(Color::hex("#DC2626"))
                            .icon_size(32.0)
                            .on_press(say("Big back")),
                    )
                    .child(CloseButton::new().enabled(false)),
            )),
        ])
    }
}
