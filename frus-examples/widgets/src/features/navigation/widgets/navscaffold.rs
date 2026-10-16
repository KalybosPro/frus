//! `NavScaffold` — navigation that adapts to the screen's size class: a bottom bar on a
//! phone, a rail on a tablet, an extended rail (or a docked drawer) on a wide window.
//! The destinations are declared once.

use frus::{
    Center, Color, Container, Icons, NavScaffold, SegmentedButton, SizeClass, State, StateContext,
    StatefulWidget, Text, Widget,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(NavScaffoldDemo.into_widget())
}

struct NavScaffoldDemo;

#[derive(Default)]
struct NavScaffoldDemoState {
    /// Index into `CLASSES`.
    class: usize,
    selected: usize,
}

const CLASSES: [(&str, SizeClass); 3] = [
    ("Compact", SizeClass::Compact),
    ("Medium", SizeClass::Medium),
    ("Expanded", SizeClass::Expanded),
];
const PLACES: [&str; 3] = ["Home", "Library", "Account"];

impl StatefulWidget for NavScaffoldDemo {
    type State = NavScaffoldDemoState;

    fn create_state(&self) -> NavScaffoldDemoState {
        NavScaffoldDemoState::default()
    }
}

impl State for NavScaffoldDemoState {
    type Widget = NavScaffoldDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        // In an app the class comes from the screen — `MediaQuery::of().size_class()` —
        // so rotating a tablet or widening a window switches the presentation. Here it
        // is picked by hand so all three fit in the demo.
        let class = CLASSES[self.class].1;
        let shell = NavScaffold::new(
            class,
            self.selected,
            cx.handler(|s: &mut Self, i: usize| s.selected = i),
        )
        .destination(Icons::HOME, "Home")
        .destination(Icons::LIBRARY_BOOKS, "Library")
        .badge(2)
        .destination(Icons::ACCOUNT_CIRCLE, "Account")
        // The body is added after the destinations.
        .body(Center::new(Text::new(PLACES[self.selected]).size(20.0)));

        let picker = CLASSES.iter().fold(
            SegmentedButton::new(self.class, cx.handler(|s: &mut Self, i: usize| s.class = i)),
            |picker, (name, _)| picker.segment(*name),
        );

        sections(vec![
            Box::new(section("Size class", picker)),
            Box::new(
                Container::new()
                    .width(320.0)
                    .height(320.0)
                    .border(1.0, Color::hex("#A1A1AA"))
                    .clip()
                    .child(shell),
            ),
        ])
    }
}
