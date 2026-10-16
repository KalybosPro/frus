//! `Wrap` and `ListBody` — a row that flows onto new lines when it runs out of room,
//! and a plain list of children one after the other.

use frus::{AlignContent, Color, Container, Justify, ListBody, Text, Widget, Wrap};

use crate::common::shared::{section, sections};

const TAGS: [&str; 10] = [
    "rust",
    "layout",
    "widgets",
    "desktop",
    "android",
    "web",
    "themes",
    "text",
    "gestures",
    "animation",
];

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        // Tags, filters, chips: as many per line as fit, the rest on the next line.
        // `gap` spaces them along a line, `run_gap` spaces the lines. Narrow the window
        // and watch them reflow.
        Box::new(section(
            "Tags that reflow",
            TAGS.iter()
                .fold(Wrap::new().gap(8.0).run_gap(8.0), |wrap, tag| {
                    wrap.child(chip(tag))
                }),
        )),
        // In a fixed width, `justify` places each line's items and `align_lines` places
        // the lines in the box.
        Box::new(section(
            "Centred lines",
            Container::new().color(Color::hex("#EEF2FF")).child(
                TAGS.iter().fold(
                    Wrap::new()
                        .width(260.0)
                        .padding(8.0)
                        .gap(6.0)
                        .run_gap(6.0)
                        .justify(Justify::Center)
                        .align_lines(AlignContent::Center),
                    |wrap, tag| wrap.child(chip(tag)),
                ),
            ),
        )),
        // `ListBody` lays its children out one after the other along an axis, each at
        // its own size — the body of a list without the scrolling.
        Box::new(section(
            "ListBody",
            ListBody::vertical()
                .gap(4.0)
                .child(row("First entry"))
                .child(row("Second entry"))
                .child(row("Third entry")),
        )),
    ])
}

fn chip(label: &str) -> Container {
    Container::new()
        .padding_each(6.0, 12.0, 6.0, 12.0)
        .radius(16.0)
        .color(Color::hex("#6366F1"))
        .child(Text::new(label).size(13.0).color(Color::hex("#FFFFFF")))
}

fn row(label: &str) -> Container {
    Container::new()
        .padding(10.0)
        .radius(6.0)
        .border(1.0, Color::hex("#C7D2FE"))
        .child(Text::new(label))
}
