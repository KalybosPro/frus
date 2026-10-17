//! `ScrollOverlay` — something drawn over a scroll region and rebuilt from its offset in
//! the same frame: a shadow under a header once content slides beneath it, a rule that
//! fills as you read. (For state that must change — loading more — use `on_scroll`.)

use frus::{Color, Column, Container, ListTile, ListView, ScrollOverlay, Text, Widget};

use crate::common::shared::{section, sections};

const HEADER: f32 = 40.0;

pub fn example() -> Box<dyn Widget> {
    // The list leaves room at its top for the header drawn over it.
    let list = ListView::new(40, 48.0, |i| {
        ListTile::new().title(format!("Chapter {}", i + 1))
    })
    .padding_each(HEADER, 0.0, 0.0, 0.0)
    .width(320.0)
    .height(300.0);

    // The builder gets the offset `(x, y)` and the overlay's box. It runs every frame the
    // list moves, so keep it cheap and give it no state of its own.
    let overlay = ScrollOverlay::new(list, |(_, y), area| {
        let shadow = (y / 24.0).clamp(0.0, 1.0) * 0.25;
        let header = Container::new()
            .width(area.width)
            .height(HEADER)
            .color(Color::hex("#F4F4F5"))
            .padding_each(10.0, 16.0, 0.0, 16.0)
            .shadows([frus::BoxShadow::new(
                0.0,
                2.0,
                6.0,
                Color::rgba(0.0, 0.0, 0.0, shadow),
            )])
            .child(Text::new(format!("Contents — {y:.0} px down")));
        Column::new().child(header)
    });

    sections(vec![Box::new(section(
        "A header whose shadow appears as the list scrolls",
        Container::new()
            .border(1.0, Color::hex("#A1A1AA"))
            .child(overlay),
    ))])
}
