//! `SearchAnchor` — a search bar that opens a **search view** when pressed: the same
//! query, and a list of suggestions under it. Full screen on a phone, a panel under the
//! bar on a desktop.

use frus::{ListTile, SearchAnchor, State, StateContext, StatefulWidget, Text, Widget};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(SearchAnchorDemo.into_widget())
}

const CITIES: [&str; 8] = [
    "Amsterdam",
    "Berlin",
    "Dakar",
    "Lisbon",
    "Lomé",
    "Nairobi",
    "Oslo",
    "Tokyo",
];

struct SearchAnchorDemo;

#[derive(Default)]
struct SearchAnchorDemoState {
    /// The view is open because the application says so.
    open: bool,
    query: String,
    chosen: Option<&'static str>,
}

impl StatefulWidget for SearchAnchorDemo {
    type State = SearchAnchorDemoState;

    fn create_state(&self) -> SearchAnchorDemoState {
        SearchAnchorDemoState::default()
    }
}

impl State for SearchAnchorDemoState {
    type Widget = SearchAnchorDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        let needle = self.query.to_lowercase();
        let anchor = CITIES
            .iter()
            .filter(|city| city.to_lowercase().contains(&needle))
            .fold(
                SearchAnchor::new(self.open, self.query.clone())
                    .hint("Search a city")
                    .on_open(cx.callback(|s: &mut SearchAnchorDemoState| s.open = true))
                    .on_close(cx.callback(|s: &mut SearchAnchorDemoState| s.open = false))
                    .on_input(cx.handler(|s: &mut SearchAnchorDemoState, t: String| s.query = t))
                    .on_clear(cx.callback(|s: &mut SearchAnchorDemoState| s.query.clear())),
                |anchor, city| {
                    let city: &'static str = city;
                    // Each suggestion is any widget; a list tile that closes the view and
                    // keeps the choice is the usual one.
                    anchor.suggestion(ListTile::new().title(city).on_tap(cx.callback(
                        move |s: &mut SearchAnchorDemoState| {
                            s.chosen = Some(city);
                            s.query = city.to_string();
                            s.open = false;
                        },
                    )))
                },
            );

        sections(vec![
            Box::new(section("Press the bar", anchor)),
            Box::new(Text::new(match self.chosen {
                Some(city) => format!("Chosen: {city}"),
                None => "No city chosen".to_string(),
            })),
        ])
    }
}
