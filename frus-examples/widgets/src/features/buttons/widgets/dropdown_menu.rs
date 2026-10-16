//! `DropdownMenu` — a dropdown that looks like a text field and filters its choices as
//! you type. For a closed list too long to scan (countries, time zones).

use frus::{DropdownMenu, State, StateContext, StatefulWidget, Text, Widget};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(DropdownMenuDemo.into_widget())
}

const COUNTRIES: [&str; 8] = [
    "Argentina",
    "Belgium",
    "Canada",
    "Denmark",
    "France",
    "Ghana",
    "Japan",
    "Kenya",
];

struct DropdownMenuDemo;

struct DropdownMenuDemoState {
    /// What is typed while the menu is open. The field shows the **selected** choice
    /// again as soon as the menu closes, so a half-typed query never looks like a value.
    query: String,
    open: bool,
    selected: Option<usize>,
}

impl StatefulWidget for DropdownMenuDemo {
    type State = DropdownMenuDemoState;

    fn create_state(&self) -> DropdownMenuDemoState {
        DropdownMenuDemoState {
            query: String::new(),
            open: false,
            selected: None,
        }
    }
}

impl State for DropdownMenuDemoState {
    type Widget = DropdownMenuDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let menu = DropdownMenu::new(
            self.query.clone(),
            self.open,
            // Typing keeps the menu open and narrows the list.
            cx.handler(|s: &mut DropdownMenuDemoState, query: String| {
                s.query = query;
                s.open = true;
            }),
            cx.callback(|s: &mut DropdownMenuDemoState| s.open = !s.open),
        )
        .label("Country")
        .placeholder("Type to filter")
        .width(260.0)
        .max_visible(5)
        .selected(self.selected)
        // The index is into the full list, whatever the filter left showing.
        .options(
            &COUNTRIES,
            cx.handler(|s: &mut DropdownMenuDemoState, i: usize| {
                s.selected = Some(i);
                s.open = false;
                s.query.clear();
            }),
        );

        sections(vec![
            Box::new(section("Filtering as you type", menu)),
            Box::new(Text::new(match self.selected {
                Some(i) => format!("Selected: {}", COUNTRIES[i]),
                None => "Nothing selected".to_string(),
            })),
        ])
    }
}
