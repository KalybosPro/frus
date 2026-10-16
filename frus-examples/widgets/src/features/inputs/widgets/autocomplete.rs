//! `Autocomplete` — a text field that suggests completions in a floating list. The
//! application filters the suggestions; the widget shows them and reports the pick.

use frus::{Autocomplete, State, StateContext, StatefulWidget, Text, Widget};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(AutocompleteDemo.into_widget())
}

const LANGUAGES: [&str; 10] = [
    "C",
    "C++",
    "Go",
    "Haskell",
    "Java",
    "JavaScript",
    "Kotlin",
    "Python",
    "Rust",
    "Swift",
];

/// The suggestions for a query: plain logic, so it is tested below.
fn suggestions(query: &str) -> Vec<&'static str> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    LANGUAGES
        .iter()
        .copied()
        .filter(|language| {
            let language = language.to_lowercase();
            language.contains(&query) && language != query
        })
        .collect()
}

struct AutocompleteDemo;

struct AutocompleteDemoState {
    query: String,
    picked: Option<String>,
}

impl StatefulWidget for AutocompleteDemo {
    type State = AutocompleteDemoState;

    fn create_state(&self) -> AutocompleteDemoState {
        AutocompleteDemoState {
            query: String::new(),
            picked: None,
        }
    }
}

impl State for AutocompleteDemoState {
    type Widget = AutocompleteDemo;

    fn build(&self, cx: &StateContext<Self>) -> Box<dyn Widget> {
        // The list floats only while it has something in it: an empty query, or an exact
        // match, shows no list.
        let field = suggestions(&self.query).into_iter().fold(
            Autocomplete::new(
                self.query.clone(),
                cx.handler(|s: &mut AutocompleteDemoState, text: String| s.query = text),
                cx.handler(|s: &mut AutocompleteDemoState, pick: String| {
                    s.query = pick.clone();
                    s.picked = Some(pick);
                }),
            )
            .width(280.0)
            .max_visible(5),
            |field, suggestion| field.suggestion(suggestion),
        );

        sections(vec![
            Box::new(section("Type a programming language (try \"a\")", field)),
            Box::new(Text::new(match &self.picked {
                Some(language) => format!("Picked: {language}"),
                None => "Nothing picked yet".to_string(),
            })),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggestions_match_anywhere_and_ignore_case() {
        assert_eq!(suggestions("JAVA"), vec!["JavaScript"]);
        assert!(suggestions("as").contains(&"Haskell"));
    }

    #[test]
    fn nothing_is_suggested_for_an_empty_query() {
        assert!(suggestions("  ").is_empty());
    }
}
