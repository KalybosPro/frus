//! Tests for `widgets::features::inputs::widgets::autocomplete`, moved out of `src/` (milestone 662).

use widgets::features::inputs::widgets::autocomplete::*;

#[test]
fn suggestions_match_anywhere_and_ignore_case() {
    assert_eq!(suggestions("JAVA"), vec!["JavaScript"]);
    assert!(suggestions("as").contains(&"Haskell"));
}

#[test]
fn nothing_is_suggested_for_an_empty_query() {
    assert!(suggestions("  ").is_empty());
}
