//! `ReorderableList` — a list whose rows can be dragged into a new order: a playlist, a
//! set of favourites, a to-do list. The order is the app's; the list reports each move.

use frus::{
    Align, Column, Container, Icon, Icons, ListTile, ReorderGrab, ReorderableList, Text, Widget,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(frus::component(|cx| {
        let songs = cx.use_state(|| {
            vec![
                "Blue in Green",
                "So What",
                "Naima",
                "Take Five",
                "Round Midnight",
            ]
        });
        let tasks = cx.use_state(|| vec!["Write the brief", "Review", "Ship it"]);

        // `on_reorder(from, to)`: `to` is where the row ends up, already accounting for
        // the row having left its old place — remove, then insert.
        let reorder = songs.clone();
        let playlist = songs.get().into_iter().enumerate().fold(
            ReorderableList::new(move |from: usize, to: usize| {
                reorder.update(|list| move_item(list, from, to))
            })
            // A grip at the trailing edge lifts the row on press; the rest of the row
            // keeps its taps and the list keeps scrolling.
            .grab(ReorderGrab::Handle)
            .gap(4.0),
            // A key per row lets the list keep track of a row as it moves.
            |list, (i, title)| {
                list.keyed_row(
                    key_of(title),
                    ListTile::new()
                        .leading(Icon::new(Icons::MUSIC_NOTE))
                        .title(title)
                        .subtitle(format!("Track {}", i + 1)),
                )
            },
        );

        // `LongPress`: hold anywhere on the row to lift it — the touch-screen default.
        let reorder_tasks = tasks.clone();
        let todo = tasks.get().into_iter().fold(
            ReorderableList::new(move |from: usize, to: usize| {
                reorder_tasks.update(|list| move_item(list, from, to))
            })
            .grab(ReorderGrab::LongPress),
            |list, task| list.keyed_row(key_of(task), ListTile::new().title(task)),
        );

        sections(vec![
            Box::new(section(
                "Drag by the handle",
                Container::new().width(320.0).child(playlist),
            )),
            Box::new(section(
                "Hold, then drag",
                Column::new()
                    .spacing(8.0)
                    .cross_axis_alignment(Align::Start)
                    .child(Container::new().width(320.0).child(todo))
                    .child(Text::new(format!("Order: {}", tasks.get().join(" → "))).wrap()),
            )),
        ])
    }))
}

/// Moves the item at `from` so that it ends up at `to`.
fn move_item<T>(list: &mut Vec<T>, from: usize, to: usize) {
    if from < list.len() {
        let item = list.remove(from);
        list.insert(to.min(list.len()), item);
    }
}

/// A stable key for a row: here the text itself, which is unique in these lists. A real
/// app uses its own id.
fn key_of(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::move_item;

    #[test]
    fn moves_down_and_up() {
        let mut list = vec!['a', 'b', 'c', 'd'];
        move_item(&mut list, 0, 2);
        assert_eq!(list, ['b', 'c', 'a', 'd']);
        move_item(&mut list, 3, 0);
        assert_eq!(list, ['d', 'b', 'c', 'a']);
    }
}
