//! `SearchBar` — the raised pill an application searches from: a borderless field with
//! room for a leading and trailing widget.

use frus::{
    Align, Color, Column, Icon, IconButton, Icons, SearchBar, State, StateContext, StatefulWidget,
    Text, Widget,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(SearchBarDemo.into_widget())
}

const FRUITS: [&str; 6] = ["Apple", "Apricot", "Banana", "Blueberry", "Cherry", "Mango"];

struct SearchBarDemo;

#[derive(Default)]
struct SearchBarDemoState {
    query: String,
    searched: String,
    other: String,
}

impl StatefulWidget for SearchBarDemo {
    type State = SearchBarDemoState;

    fn create_state(&self) -> SearchBarDemoState {
        SearchBarDemoState::default()
    }
}

impl State for SearchBarDemoState {
    type Widget = SearchBarDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let mut bar = SearchBar::new(&self.query)
            .hint("Search fruit")
            .leading(Icon::new(Icons::SEARCH))
            .on_input(cx.handler(|s: &mut SearchBarDemoState, t: String| s.query = t))
            // Enter confirms the search.
            .on_submit(cx.callback(|s: &mut SearchBarDemoState| s.searched = s.query.clone()))
            .max_width(320.0);
        // A clear button, only when there is something to clear.
        if !self.query.is_empty() {
            bar = bar.trailing(
                IconButton::new(Icons::CLOSE)
                    .label("Clear")
                    .on_press(cx.callback(|s: &mut SearchBarDemoState| s.query.clear())),
            );
        }

        // The results are filtered live from the query.
        let needle = self.query.to_lowercase();
        let results = FRUITS
            .iter()
            .filter(|fruit| fruit.to_lowercase().contains(&needle))
            .fold(
                Column::new()
                    .cross_axis_alignment(Align::Start)
                    .spacing(4.0),
                |column, fruit| column.child(Text::new(*fruit)),
            );

        sections(vec![
            Box::new(section("Live filter", bar)),
            Box::new(results),
            Box::new(Text::new(format!("Last submitted: \"{}\"", self.searched))),
            // Fill, outline, corners and elevation are all overridable.
            Box::new(section(
                "Custom look",
                SearchBar::new(&self.other)
                    .hint("Flat, outlined, square-ish")
                    .elevation(0.0)
                    .background_color(Color::hex("#F1F5F9"))
                    .side(1.0, Color::hex("#94A3B8"))
                    .radius(8.0)
                    .max_width(320.0)
                    .on_input(cx.handler(|s: &mut SearchBarDemoState, t: String| s.other = t)),
            )),
        ])
    }
}
