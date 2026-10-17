//! `Container` — one box that can size, pad, colour, round, border and shadow its child.

use frus::{BorderRadius, BoxDecoration, BoxShadow, Color, Colors, Container, Row, Text, Widget};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    sections(vec![
        // The simplest use: a fixed size and a colour.
        Box::new(section(
            "Size and colour",
            Container::new()
                .width(120.0)
                .height(60.0)
                .color(Color::hex("#6366F1")),
        )),
        // Padding is inside the box, margin outside it; `radius` rounds the corners.
        Box::new(section(
            "Padding, margin and radius",
            Container::new()
                .padding(16.0)
                .margin(8.0)
                .radius(12.0)
                .color(Color::hex("#E0E7FF"))
                .child(Text::new("Padded content").color(Color::hex("#312E81"))),
        )),
        // A border, and a shadow under it.
        Box::new(section(
            "Border and shadow",
            Container::new()
                .padding(16.0)
                .radius(8.0)
                .color(Colors::WHITE)
                .border(1.0, Color::hex("#E4E4E7"))
                .shadows([BoxShadow::new(
                    0.0,
                    4.0,
                    12.0,
                    Color::rgba(0.0, 0.0, 0.0, 0.15),
                )])
                .child(Text::new("A card-like box").color(Color::hex("#09090B"))),
        )),
        // `decoration` takes the whole description at once — what a theme or a design
        // system would hand you.
        Box::new(section(
            "BoxDecoration",
            Row::new()
                .spacing(12.0)
                .child(
                    Container::new().width(64.0).height(64.0).decoration(
                        BoxDecoration::filled(Color::hex("#F59E0B"))
                            .radius(BorderRadius::uniform(32.0)),
                    ),
                )
                .child(Container::new().width(64.0).height(64.0).decoration(
                    BoxDecoration::filled(Color::hex("#10B981")).radius(BorderRadius::uniform(4.0)),
                )),
        )),
    ])
}
