//! `RotatedBox` — turns its child by whole quarter turns, and the layout turns with it.
//!
//! The difference from `Transform::rotate` is the layout: a rotated box swaps its width
//! and height, so its neighbours make room for the turned child. A transform only moves
//! the pixels, and the neighbours still see the unturned box.

use frus::{Align, Color, Container, Insets, MainAxisSize, Padding, RotatedBox, Row, Text, Widget};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    sections(vec![
        // 0 to 3 quarter turns, clockwise. The odd ones are taller than wide.
        Box::new(section(
            "Quarter turns",
            Row::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(Align::Start)
                .spacing(12.0)
                .child(RotatedBox::new(0).child(label("0 turns")))
                .child(RotatedBox::new(1).child(label("1 turn")))
                .child(RotatedBox::new(2).child(label("2 turns")))
                .child(RotatedBox::new(3).child(label("3 turns"))),
        )),
        // The usual use: a vertical caption beside content, taking only the width of
        // one line.
        Box::new(section(
            "Vertical caption",
            Row::new()
                .main_axis_size(MainAxisSize::Min)
                .spacing(8.0)
                .child(RotatedBox::new(3).child(Padding::new(
                    Insets::symmetric(4.0, 0.0),
                    Text::new("SALES 2026").size(12.0),
                )))
                .child(
                    Container::new()
                        .width(200.0)
                        .height(100.0)
                        .radius(6.0)
                        .color(Color::hex("#E0E7FF")),
                ),
        )),
    ])
}

fn label(text: &str) -> Container {
    Container::new()
        .padding_each(6.0, 12.0, 6.0, 12.0)
        .radius(6.0)
        .color(Color::hex("#6366F1"))
        .child(Text::new(text).color(Color::hex("#FFFFFF")))
}
