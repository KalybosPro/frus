//! `Expanded`, `Flexible` and `Spacer` — share the free room of a `Row` or a `Column`
//! between some of its children.

use frus::{Alignment, Color, Container, Expanded, Flexible, Row, Spacer, Text, Widget};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    sections(vec![
        // `Expanded` takes whatever room the fixed children leave: the classic "label
        // fills, button keeps its size" row.
        Box::new(section(
            "Expanded fills the rest",
            Row::new()
                .spacing(8.0)
                .child(fixed("fixed"))
                .child(Expanded::new(filler("Expanded", "#6366F1")))
                .child(fixed("fixed")),
        )),
        // Several expanded children split the room by their `flex` factors: 1 : 2 : 1.
        Box::new(section(
            "Flex factors",
            Row::new()
                .spacing(8.0)
                .child(Expanded::new(filler("1", "#818CF8")))
                .child(Expanded::new(filler("2", "#6366F1")).flex(2.0))
                .child(Expanded::new(filler("1", "#818CF8"))),
        )),
        // `Flexible` may take up to its share but does not have to: a short child keeps
        // its own width. `Expanded` beside it gets what the flexible one did not use.
        Box::new(section(
            "Flexible keeps its own size",
            Row::new()
                .spacing(8.0)
                .child(Flexible::new(
                    Container::new()
                        .padding(8.0)
                        .color(Color::hex("#F59E0B"))
                        .child(Text::new("Flexible").color(Color::hex("#FFFFFF"))),
                ))
                .child(Expanded::new(filler("Expanded", "#6366F1"))),
        )),
        // A long text in a `Flexible` wraps instead of pushing its neighbours off the
        // row — the usual cure for a row that overflows.
        Box::new(section(
            "Flexible wraps long text",
            Row::new()
                .spacing(8.0)
                .child(fixed("icon"))
                .child(Flexible::new(
                    Text::new(
                        "A long description that would run past the edge of the row if it \
                         were not allowed to wrap inside a Flexible.",
                    )
                    .wrap(),
                )),
        )),
        // `Spacer` is an empty `Expanded`: it pushes what follows it to the end.
        Box::new(section(
            "Spacer",
            Container::new()
                .padding(8.0)
                .color(Color::hex("#EEF2FF"))
                .child(
                    Row::new()
                        .child(fixed("left"))
                        .child(Spacer::new())
                        .child(fixed("right")),
                ),
        )),
    ])
}

fn fixed(label: &str) -> Container {
    Container::new()
        .width(64.0)
        .height(40.0)
        .radius(6.0)
        .color(Color::hex("#10B981"))
        .alignment(Alignment::CENTER)
        .child(Text::new(label).size(13.0).color(Color::hex("#FFFFFF")))
}

fn filler(label: &str, color: &str) -> Container {
    Container::new()
        .height(40.0)
        .radius(6.0)
        .color(Color::hex(color))
        .alignment(Alignment::CENTER)
        .child(Text::new(label).size(13.0).color(Color::hex("#FFFFFF")))
}
