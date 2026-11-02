//! `BottomNavigationBar` — the classic bar of three to five destinations along the bottom
//! of a phone screen. A fixed bar keeps every item the same width; a shifting one widens
//! the selected item, shows only its label and takes its colour.

use frus::{
    Align, BottomNavigationBar, BottomNavigationBarItem, BottomNavigationBarType, Center, Color,
    Column, Container, Expanded, Icons, State, StateContext, StatefulWidget, Text, Widget,
};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(BottomNavigationBarDemo.into_widget())
}

/// The selected item of each bar is the demo's state: the bar shows what it is told and
/// reports the item tapped.
struct BottomNavigationBarDemo;

#[derive(Default)]
struct BottomNavigationBarDemoState {
    fixed: usize,
    shifting: usize,
}

impl StatefulWidget for BottomNavigationBarDemo {
    type State = BottomNavigationBarDemoState;

    fn create_state(&self) -> BottomNavigationBarDemoState {
        BottomNavigationBarDemoState::default()
    }
}

const FIXED: [&str; 3] = ["Home", "Search", "Profile"];
const SHIFTING: [&str; 4] = ["Music", "Library", "Search", "Profile"];

impl State for BottomNavigationBarDemoState {
    type Widget = BottomNavigationBarDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        // Three items: a fixed bar. Every label shows; the selected item takes the
        // selected colour, and an item may have an icon of its own for when it is selected.
        let fixed =
            BottomNavigationBar::new(self.fixed, cx.handler(|s: &mut Self, i: usize| s.fixed = i))
                .item(BottomNavigationBarItem::new(Icons::HOME, "Home"))
                .item(BottomNavigationBarItem::new(Icons::SEARCH, "Search"))
                .item(
                    BottomNavigationBarItem::new(Icons::PERSON_OUTLINE, "Profile")
                        .active_icon(Icons::PERSON)
                        .tooltip("Your profile"),
                )
                .selected_item_color(Color::hex("#7C3AED"));

        // Four items: a shifting bar. Tap one and its colour spreads from it across the bar.
        let shifting = BottomNavigationBar::new(
            self.shifting,
            cx.handler(|s: &mut Self, i: usize| s.shifting = i),
        )
        .kind(BottomNavigationBarType::Shifting)
        .items([
            BottomNavigationBarItem::new(Icons::MUSIC_NOTE, "Music")
                .background_color(Color::hex("#0F766E")),
            BottomNavigationBarItem::new(Icons::LIBRARY_MUSIC, "Library")
                .background_color(Color::hex("#B45309")),
            BottomNavigationBarItem::new(Icons::SEARCH, "Search")
                .background_color(Color::hex("#1D4ED8")),
            BottomNavigationBarItem::new(Icons::PERSON_OUTLINE, "Profile")
                .background_color(Color::hex("#BE123C")),
        ]);

        sections(vec![
            Box::new(section("Fixed", phone(FIXED[self.fixed], fixed))),
            Box::new(section(
                "Shifting",
                phone(SHIFTING[self.shifting], shifting),
            )),
        ])
    }
}

/// A small phone-shaped frame: the page above, the bar along the bottom.
fn phone(page: &str, bar: BottomNavigationBar) -> Container {
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
