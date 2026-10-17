//! Tests for `widgets::features::display::widgets::tree`, moved out of `src/` (milestone 662).

use std::collections::HashSet;
use widgets::features::display::widgets::tree::*;

#[test]
fn collapsed_nodes_hide_their_children() {
    let rows = visible_rows(&HashSet::new());
    let labels: Vec<_> = rows.iter().map(|r| r.2).collect();
    assert_eq!(labels, ["src", "assets", "Cargo.toml", "README.md"]);
}

#[test]
fn expanded_nodes_show_children_one_level_deeper() {
    let rows = visible_rows(&HashSet::from([1, 2]));
    assert_eq!(rows[1], (2, 1, "features", true, true));
    assert_eq!(rows[2], (3, 2, "display.rs", false, false));
}
