//! `Dismissible` — swipe a row aside to remove it (or archive it). The row slides out,
//! its gap closes, then the app is told and drops the item from its list.

use frus::{
    keyed, Align, Button, Color, Colors, Column, Container, Icon, Icons, Justify, ListTile, Row,
    Text, Variant, Widget,
};

use crate::common::shared::{section, sections};

const INBOX: [&str; 5] = [
    "Lunch on Friday?",
    "Your invoice is ready",
    "Weekly report",
    "New comment on your post",
    "Password changed",
];

pub fn example() -> Box<dyn Widget> {
    Box::new(frus::component(|cx| {
        // Indices into `INBOX` still shown, and what happened last.
        let mail = cx.use_state(|| (0..INBOX.len()).collect::<Vec<usize>>());
        let last = cx.use_state(|| "Swipe a message left or right".to_string());

        let rows = mail.get().into_iter().fold(
            Column::new().cross_axis_alignment(Align::Stretch),
            |column, id| {
                let (archive, delete) = (mail.clone(), mail.clone());
                let (said_archive, said_delete) = (last.clone(), last.clone());
                // One message per direction. Swiping towards the end (rightwards here)
                // shows `background`; the other way shows `secondary_background`.
                let row = frus::Dismissible::new(
                    Container::new().color(Colors::WHITE).child(
                        ListTile::new()
                            .leading(Icon::new(Icons::MAIL))
                            .title(INBOX[id]),
                    ),
                )
                .width(320.0)
                .height(56.0)
                .background(strip(Icons::ARCHIVE, "Archive", "#16A34A", Justify::Start))
                .secondary_background(strip(Icons::DELETE, "Delete", "#DC2626", Justify::End))
                .on_dismiss_to_end(move || {
                    archive.update(|m| m.retain(|&i| i != id));
                    said_archive.set(format!("Archived “{}”", INBOX[id]));
                })
                .on_dismiss_to_start(move || {
                    delete.update(|m| m.retain(|&i| i != id));
                    said_delete.set(format!("Deleted “{}”", INBOX[id]));
                });
                // A key per item: when a row goes, the ones after it move up, and the key
                // keeps each row's swipe state with its own message.
                column.child(keyed(id, row))
            },
        );

        let restore = mail.clone();
        sections(vec![
            Box::new(section("Inbox", Container::new().width(320.0).child(rows))),
            Box::new(Text::new(last.get()).color(cx.theme().muted)),
            Box::new(
                Button::new("Restore all")
                    .variant(Variant::Outlined)
                    .on_press(move || restore.set((0..INBOX.len()).collect())),
            ),
        ])
    }))
}

/// What shows behind a row as it slides: a colour, an icon and a word.
fn strip(icon: frus::IconData, label: &str, hex: &str, side: Justify) -> Container {
    Container::new()
        .color(Color::hex(hex))
        .padding_each(0.0, 16.0, 0.0, 16.0)
        .child(
            Row::new()
                .main_axis_alignment(side)
                .spacing(8.0)
                .child(Icon::new(icon).color(Colors::WHITE))
                .child(Text::new(label).color(Colors::WHITE)),
        )
}
