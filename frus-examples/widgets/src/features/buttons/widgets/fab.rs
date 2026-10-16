//! `FloatingActionButton` — the one action a screen is for, floating above it. Put it in
//! a `Scaffold` with `.fab(..)`; here the buttons sit in the page so all sizes show.

use frus::{
    Align, Color, FloatingActionButton, Icons, Row, State, StateContext, StatefulWidget, Text,
    Widget, Wrap,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(FabDemo.into_widget())
}

struct FabDemo;

struct FabDemoState {
    created: u32,
}

impl StatefulWidget for FabDemo {
    type State = FabDemoState;

    fn create_state(&self) -> FabDemoState {
        FabDemoState { created: 0 }
    }
}

impl State for FabDemoState {
    type Widget = FabDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let create = || cx.callback(|s: &mut FabDemoState| s.created += 1);

        sections(vec![
            Box::new(Text::new(format!("{} items created", self.created))),
            // Small for a secondary action next to another one, regular by default,
            // large when the screen is mostly empty and the action is everything.
            Box::new(section(
                "Sizes",
                Row::new()
                    .spacing(16.0)
                    .cross_axis_alignment(Align::Center)
                    .child(
                        FloatingActionButton::new(Icons::ADD)
                            .small()
                            .on_press(create()),
                    )
                    .child(FloatingActionButton::new(Icons::ADD).on_press(create()))
                    .child(
                        FloatingActionButton::new(Icons::ADD)
                            .large()
                            .on_press(create()),
                    ),
            )),
            // Extended: words when the icon alone would not say what happens.
            Box::new(section(
                "Extended",
                Wrap::new()
                    .gap(12.0)
                    .run_gap(12.0)
                    .child(
                        FloatingActionButton::extended("Compose")
                            .icon(Icons::EDIT)
                            .on_press(create()),
                    )
                    .child(FloatingActionButton::extended("New item").on_press(create())),
            )),
            // Colours, shape and elevation override the theme's.
            Box::new(section(
                "Custom colours and shape",
                Row::new()
                    .spacing(16.0)
                    .child(
                        FloatingActionButton::new(Icons::NAVIGATION)
                            .background(Color::hex("#1E3A8A"))
                            .foreground(Color::hex("#FFFFFF"))
                            .radius(28.0)
                            .elevation(6.0)
                            .on_press(create()),
                    )
                    .child(FloatingActionButton::new(Icons::DELETE).enabled(false)),
            )),
        ])
    }
}
