//! `BottomBar` — the bar of three to five main destinations along the bottom of a phone
//! screen. One is selected; a badge can say something is waiting in another.

use frus::{
    Align, BottomBar, Center, Color, Column, Container, Expanded, Icons, NavigationDestination,
    RailLabels, State, StateContext, StatefulWidget, Text, Widget,
};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(BottomBarDemo.into_widget())
}

/// A small `StatefulWidget`: the selected destination of each bar is its state, and
/// `cx.handler` turns "the bar reports an index" into "this method changes the state".
struct BottomBarDemo;

#[derive(Default)]
struct BottomBarDemoState {
    first: usize,
    second: usize,
}

impl StatefulWidget for BottomBarDemo {
    type State = BottomBarDemoState;

    fn create_state(&self) -> BottomBarDemoState {
        BottomBarDemoState::default()
    }
}

const PLACES: [&str; 4] = ["Home", "Explore", "Inbox", "Profile"];

impl State for BottomBarDemoState {
    type Widget = BottomBarDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        // `item(icon, label)` adds a destination; the builders after it (`badge`,
        // `selected_icon`) apply to the last one added. An outlined icon at rest and the
        // filled one when selected is how the bar says where you are.
        let simple = BottomBar::new(self.first, cx.handler(|s: &mut Self, i: usize| s.first = i))
            .item(Icons::HOME, "Home")
            .item(Icons::EXPLORE, "Explore")
            .item(Icons::MAIL, "Inbox")
            .badge(3)
            .item(Icons::PERSON, "Profile");

        // The same destinations can be declared once as values and handed to any of the
        // navigation widgets (bar, rail, drawer). Here only the selected one is labelled.
        let destinations = vec![
            NavigationDestination::new(Icons::FAVORITE_BORDER, "Saved")
                .selected_icon(Icons::FAVORITE),
            NavigationDestination::new(Icons::MAIL_OUTLINE, "Inbox").badge(12),
            NavigationDestination::new(Icons::SETTINGS, "Settings"),
        ];
        let labelled = BottomBar::new(
            self.second,
            cx.handler(|s: &mut Self, i: usize| s.second = i),
        )
        .destinations(destinations)
        .labels(RailLabels::Selected)
        .background(Color::hex("#EEF2FF"));

        sections(vec![
            Box::new(section(
                "Items with a badge",
                phone(PLACES[self.first], simple),
            )),
            Box::new(section(
                "Declared destinations, label on the selected one",
                phone(["Saved", "Inbox", "Settings"][self.second], labelled),
            )),
        ])
    }
}

/// A small phone-shaped frame: the page above, the bar along the bottom.
fn phone(page: &str, bar: BottomBar) -> Container {
    Container::new()
        .width(320.0)
        .height(200.0)
        .border(1.0, Color::hex("#A1A1AA"))
        .radius(12.0)
        .clip()
        .child(
            Column::new()
                .cross_axis_alignment(Align::Stretch)
                .child(Expanded::new(Center::new(Text::new(page).size(20.0))))
                .child(bar),
        )
}
