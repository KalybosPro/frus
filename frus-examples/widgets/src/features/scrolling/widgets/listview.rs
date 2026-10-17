//! `ListView` — a virtualised list: only the rows on screen are built, laid out and
//! painted, so ten thousand rows cost what ten do. Every row has the same extent.

use frus::{
    Axis, CircleAvatar, Color, Column, Container, Icon, Icons, ListTile, ListView, Text, Widget,
};

use crate::common::shared::{section, sections};

const COUNT: usize = 10_000;

pub fn example() -> Box<dyn Widget> {
    Box::new(frus::component(|cx| {
        let tapped = cx.use_state(|| None::<usize>);

        // `ListView::new(count, extent, build)`: `build(index)` is called only for the
        // rows in view, every frame they are in view. So a row is a plain widget made
        // from the index — not a component with hooks of its own; state that a row
        // shows lives above the list, like `tapped` here.
        let on_tap = tapped.clone();
        let selected = tapped.get();
        let long_list = ListView::new(COUNT, 56.0, move |i| {
            let on_tap = on_tap.clone();
            ListTile::new()
                .leading(Icon::new(Icons::RECEIPT))
                .title(format!("Invoice #{}", i + 1))
                .subtitle(format!("{:.2} €", (i as f32 * 7.31) % 500.0))
                .selected(selected == Some(i))
                .on_tap(move || on_tap.set(Some(i)))
        })
        // A list is 200 px tall unless told otherwise; inside a scrolling page it must
        // have a bounded height.
        .height(280.0)
        .width(320.0);

        // `axis(Axis::Horizontal)` turns it on its side: the extent becomes a width.
        let shelf = ListView::new(50, 72.0, |i| {
            Container::new().padding(4.0).child(
                Column::new()
                    .spacing(4.0)
                    .child(CircleAvatar::initials(format!("User {}", i + 1)))
                    .child(Text::new(format!("#{}", i + 1)).size(12.0)),
            )
        })
        .axis(Axis::Horizontal)
        .height(80.0)
        .width(320.0);

        // `reverse()` starts at the end — the newest message at the bottom of a chat.
        let chat = ListView::new(30, 40.0, |i| {
            Container::new()
                .padding_each(4.0, 12.0, 4.0, 12.0)
                .child(Text::new(format!("Message {}", 30 - i)))
        })
        .reverse()
        .height(160.0)
        .width(320.0);

        sections(vec![
            Box::new(section("10 000 rows", framed(long_list))),
            Box::new(Text::new(match tapped.get() {
                Some(i) => format!("Tapped invoice #{}", i + 1),
                None => "Tap a row".to_string(),
            })),
            Box::new(section("Horizontal", framed(shelf))),
            Box::new(section("Reversed (starts at the end)", framed(chat))),
        ])
    }))
}

/// A thin outline, so the list's bounds show.
fn framed(list: ListView) -> Container {
    Container::new()
        .border(1.0, Color::hex("#A1A1AA"))
        .child(list)
}
