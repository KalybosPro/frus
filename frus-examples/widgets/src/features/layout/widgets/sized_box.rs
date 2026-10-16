//! `SizedBox` — a box of a given size: to size its child, or, empty, as a fixed gap.

use frus::{Alignment, Color, Container, Row, SizedBox, Text, Widget};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        // A fixed width and height forces the child to that size.
        Box::new(section(
            "Width and height",
            SizedBox::new(filler("160 × 48")).width(160.0).height(48.0),
        )),
        // `square` is the same on both axes — an avatar, an icon slot.
        Box::new(section("Square", SizedBox::square(64.0, filler("64")))),
        // An empty one is a fixed gap: between two children of a row that has no
        // `spacing`, for instance.
        Box::new(section(
            "Empty, as a gap",
            Row::new()
                .child(SizedBox::square(40.0, filler("A")))
                .child(SizedBox::empty().width(48.0))
                .child(SizedBox::square(40.0, filler("B"))),
        )),
        // A fraction of the room its parent has: half of a 300 px frame.
        Box::new(section(
            "Fraction of the width",
            Container::new()
                .width(300.0)
                .border(1.0, Color::hex("#A5B4FC"))
                .child(
                    SizedBox::new(filler("half the frame"))
                        .width_fraction(0.5)
                        .height(40.0),
                ),
        )),
        // `expand` takes all the room it is offered; `shrink` takes only what its child
        // needs, inside a parent that would otherwise stretch it.
        Box::new(section(
            "Expand and shrink",
            Row::new()
                .spacing(12.0)
                .child(frame(SizedBox::expand(tag("expand"))))
                .child(frame(SizedBox::shrink(tag("shrink")))),
        )),
    ])
}

fn frame(child: impl Widget + 'static) -> Container {
    Container::new()
        .width(140.0)
        .height(64.0)
        .border(1.0, Color::hex("#A5B4FC"))
        .child(child)
}

fn filler(label: &str) -> Container {
    Container::new()
        .color(Color::hex("#6366F1"))
        .alignment(Alignment::CENTER)
        .child(Text::new(label).size(13.0).color(Color::hex("#FFFFFF")))
}

/// A label with no size of its own: it is as big as whatever sizes it.
fn tag(label: &str) -> Container {
    Container::new()
        .padding(8.0)
        .color(Color::hex("#6366F1"))
        .child(Text::new(label).size(13.0).color(Color::hex("#FFFFFF")))
}
