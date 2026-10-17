//! `Flex` — the flexbox container under `Row` and `Column`: a line of children that
//! shrink-wraps, stretches them across, and can wrap or run in reverse.
//!
//! Reach for `Row`/`Column` first. `Flex` is for when you want flexbox's own rules: a
//! container as big as its content, children stretched across it, `gap` and padding
//! built in.

use frus::{Align, Color, Container, Flex, Justify, Text, Widget};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    sections(vec![
        // A flex row is as wide as its children; `gap` spaces them and `padding` pads the
        // whole line.
        Box::new(section(
            "Row with gap and padding",
            tinted(
                Flex::row()
                    .gap(8.0)
                    .padding(8.0)
                    .child(chip("one"))
                    .child(chip("two"))
                    .child(chip("three")),
            ),
        )),
        // Children are stretched across the line by default — here, to the tallest one.
        // `align` picks another cross-axis position.
        Box::new(section(
            "Stretched across (the default) and centred",
            Flex::row()
                .gap(24.0)
                .child(tinted(
                    Flex::row().gap(4.0).child(chip("short")).child(tall_chip()),
                ))
                .child(tinted(
                    Flex::row()
                        .gap(4.0)
                        .align(Align::Center)
                        .child(chip("short"))
                        .child(tall_chip()),
                )),
        )),
        // With a fixed width, `justify` distributes the spare room along the line.
        Box::new(section(
            "Fixed width, justified",
            tinted(
                Flex::row()
                    .width(300.0)
                    .justify(Justify::SpaceBetween)
                    .padding(8.0)
                    .child(chip("left"))
                    .child(chip("middle"))
                    .child(chip("right")),
            ),
        )),
        // `reverse` lays the children out from the far end — a chat transcript that
        // grows upwards is a reversed column.
        Box::new(section(
            "Reversed column",
            tinted(
                Flex::column()
                    .reverse()
                    .gap(4.0)
                    .padding(8.0)
                    .child(chip("first child"))
                    .child(chip("second child"))
                    .child(chip("third child")),
            ),
        )),
        // `wrap` moves what does not fit onto a new line; `run_gap` spaces the lines.
        Box::new(section(
            "Wrapping",
            tinted(
                (1..=12).fold(
                    Flex::row()
                        .wrap()
                        .width(280.0)
                        .gap(6.0)
                        .run_gap(6.0)
                        .padding(8.0),
                    |flex, i| flex.child(chip(&format!("item {i}"))),
                ),
            ),
        )),
    ])
}

fn tinted(child: impl Widget + 'static) -> Container {
    Container::new()
        .radius(8.0)
        .color(Color::hex("#E0E7FF"))
        .child(child)
}

fn chip(label: &str) -> Container {
    Container::new()
        .padding_each(6.0, 10.0, 6.0, 10.0)
        .radius(6.0)
        .color(Color::hex("#6366F1"))
        .child(Text::new(label).size(13.0).color(Color::hex("#FFFFFF")))
}

fn tall_chip() -> Container {
    Container::new()
        .padding(10.0)
        .height(72.0)
        .radius(6.0)
        .color(Color::hex("#4338CA"))
        .child(Text::new("tall").size(13.0).color(Color::hex("#FFFFFF")))
}
