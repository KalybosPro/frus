//! Tests for `widgets::common::catalog`, moved out of `src/` (milestone 664).

use std::collections::HashSet;

use widgets::common::catalog::*;

/// A demo is found by its id alone, so two with the same id would hide one another.
#[test]
fn demo_ids_are_unique() {
    let mut seen = HashSet::new();
    for category in categories() {
        for demo in (category.demos)() {
            assert!(seen.insert(demo.id), "two demos are called `{}`", demo.id);
        }
    }
}
