//! `Kanban` — a board of columns and cards, with drag and drop between columns. The
//! board is controlled: the app holds the cards and moves one when a drop is reported.

use frus::{Align, Axis, Column, Container, Kanban, SingleChildScrollView, Text, Widget};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(frus::component(|cx| {
        let board = cx.use_state(|| {
            vec![
                (
                    "To do",
                    vec!["Design the icon", "Write the docs", "Plan the release"],
                ),
                ("Doing", vec!["Fix the scroll bug"]),
                ("Done", vec!["Set up CI"]),
            ]
            .into_iter()
            .map(|(title, cards)| {
                (
                    title.to_string(),
                    cards.into_iter().map(String::from).collect(),
                )
            })
            .collect::<Vec<(String, Vec<String>)>>()
        });

        // `on_move(from_col, from_pos, to_col, to_pos)` fires on a drop; `on_add(col)`
        // when a column's "+ Add card" is pressed. Both just change the app's lists.
        let (moved, added) = (board.clone(), board.clone());
        let kanban = board.get().into_iter().fold(
            Kanban::new(
                move |from_col: usize, from_pos: usize, to_col: usize, to_pos: usize| {
                    moved.update(|columns| {
                        move_card(columns, (from_col, from_pos), (to_col, to_pos))
                    })
                },
            )
            .on_add(move |col: usize| {
                added.update(|columns| {
                    let n = columns[col].1.len() + 1;
                    columns[col].1.push(format!("New card {n}"));
                })
            }),
            |kanban, (title, cards)| kanban.column(title, cards),
        );

        let counts: Vec<String> = board
            .get()
            .iter()
            .map(|(title, cards)| format!("{title}: {}", cards.len()))
            .collect();

        // A board is wider than a phone: let it scroll sideways.
        sections(vec![Box::new(section(
            "Drag cards between columns",
            Column::new()
                .spacing(8.0)
                .cross_axis_alignment(Align::Start)
                .child(
                    SingleChildScrollView::new()
                        .axis(Axis::Horizontal)
                        .width(320.0)
                        .child(Container::new().width(700.0).child(kanban)),
                )
                .child(Text::new(counts.join(" · ")).color(cx.theme().muted)),
        ))])
    }))
}

/// Takes the card at `from` and inserts it at `to` (positions within columns).
fn move_card(columns: &mut [(String, Vec<String>)], from: (usize, usize), to: (usize, usize)) {
    let Some(card) = columns
        .get_mut(from.0)
        .filter(|(_, cards)| from.1 < cards.len())
        .map(|(_, cards)| cards.remove(from.1))
    else {
        return;
    };
    if let Some((_, cards)) = columns.get_mut(to.0) {
        // Within one column the removal shifted everything after it up by one.
        let at = if from.0 == to.0 && to.1 > from.1 {
            to.1 - 1
        } else {
            to.1
        };
        cards.insert(at.min(cards.len()), card);
    }
}

#[cfg(test)]
mod tests {
    use super::move_card;

    fn board() -> Vec<(String, Vec<String>)> {
        vec![
            ("A".into(), vec!["1".into(), "2".into()]),
            ("B".into(), vec!["3".into()]),
        ]
    }

    #[test]
    fn moves_a_card_to_another_column() {
        let mut columns = board();
        move_card(&mut columns, (0, 0), (1, 1));
        assert_eq!(columns[0].1, ["2"]);
        assert_eq!(columns[1].1, ["3", "1"]);
    }

    #[test]
    fn a_bad_source_changes_nothing() {
        let mut columns = board();
        move_card(&mut columns, (0, 9), (1, 0));
        assert_eq!(columns, board());
    }
}
