//! `Tree` — a hierarchy (files, categories, an org chart) shown as indented rows that
//! expand and collapse. The app owns the structure and which nodes are open, and hands
//! the tree only the rows that are visible.

use std::collections::HashSet;

use frus::{Align, Column, Text, Tree, Widget};

/// One node: its id, its parent's id (`None` at the root), and its label.
const NODES: [(u64, Option<u64>, &str); 9] = [
    (1, None, "src"),
    (2, Some(1), "features"),
    (3, Some(2), "display.rs"),
    (4, Some(2), "navigation.rs"),
    (5, Some(1), "lib.rs"),
    (6, None, "assets"),
    (7, Some(6), "logo.png"),
    (8, None, "Cargo.toml"),
    (9, None, "README.md"),
];

/// A visible row: `(id, depth, label, has children, is expanded)`, what `Tree::node` takes.
type VisibleRow = (u64, usize, &'static str, bool, bool);

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(frus::component(|cx| {
        let expanded = cx.use_state(|| HashSet::from([1u64]));
        let selected = cx.use_state(|| None::<u64>);

        let toggle = expanded.clone();
        let select = selected.clone();
        // The chevron toggles a node; a click anywhere else on the row selects it.
        let tree = expanded.with(visible_rows).into_iter().fold(
            Tree::new(move |id: u64| {
                toggle.update(|open| {
                    if !open.remove(&id) {
                        open.insert(id);
                    }
                })
            })
            .on_select(move |id: u64| select.set(Some(id)))
            .selected(selected.get()),
            |tree, (id, depth, label, expandable, open)| {
                tree.node(id, depth, label, expandable, open)
            },
        );

        let label = selected
            .get()
            .and_then(|id| NODES.iter().find(|(n, ..)| *n == id))
            .map_or("Nothing selected".to_string(), |(_, _, name)| {
                format!("Selected: {name}")
            });

        Box::new(
            Column::new()
                .spacing(12.0)
                .cross_axis_alignment(Align::Start)
                .child(frus::Container::new().width(300.0).child(tree))
                .child(Text::new(label).color(cx.theme().muted)),
        )
    }))
}

/// The rows to show: every root, and the children of every expanded node, depth first.
fn visible_rows(expanded: &HashSet<u64>) -> Vec<VisibleRow> {
    fn walk(parent: Option<u64>, depth: usize, expanded: &HashSet<u64>, out: &mut Vec<VisibleRow>) {
        for &(id, _, label) in NODES.iter().filter(|(_, p, _)| *p == parent) {
            let has_children = NODES.iter().any(|(_, p, _)| *p == Some(id));
            let open = expanded.contains(&id);
            out.push((id, depth, label, has_children, open));
            if has_children && open {
                walk(Some(id), depth + 1, expanded, out);
            }
        }
    }
    let mut rows = Vec::new();
    walk(None, 0, expanded, &mut rows);
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
