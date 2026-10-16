//! `AspectRatio` — a box whose height follows from its width: a video frame, a photo
//! card, a square tile.

use frus::{Alignment, AspectRatio, Color, Container, Row, SizedBox, Text, Widget};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        // The box takes the width it is given and derives its height: 16:9 at 300 px
        // wide is about 169 px tall. The ratio is width / height.
        Box::new(section(
            "16 : 9",
            SizedBox::new(AspectRatio::new(16.0 / 9.0).child(frame("16 : 9", "#6366F1")))
                .width(300.0),
        )),
        // The same rule gives a square, or a box taller than it is wide.
        Box::new(section(
            "Square and portrait",
            Row::new()
                .spacing(12.0)
                .child(
                    SizedBox::new(AspectRatio::new(1.0).child(frame("1 : 1", "#10B981")))
                        .width(120.0),
                )
                .child(
                    SizedBox::new(AspectRatio::new(3.0 / 4.0).child(frame("3 : 4", "#F59E0B")))
                        .width(120.0),
                ),
        )),
    ])
}

fn frame(label: &str, color: &str) -> Container {
    Container::new()
        .radius(8.0)
        .color(Color::hex(color))
        .alignment(Alignment::CENTER)
        .child(Text::new(label).color(Color::hex("#FFFFFF")))
}
