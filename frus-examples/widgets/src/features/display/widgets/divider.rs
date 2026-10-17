//! `Divider` and `VerticalDivider` — a thin rule in the theme's colours, across a column
//! or down a row.

use frus::{Align, Color, Column, Container, Divider, Row, Text, VerticalDivider, Widget};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    sections(vec![
        // A divider is as wide as it is allowed to be, inside a band of `height` (16 by
        // default) that leaves room above and below. A column centres its children
        // unless told to stretch them, so `Align::Stretch` is what lets it run across.
        Box::new(section(
            "Divider",
            Container::new().width(300.0).child(
                Column::new()
                    .cross_axis_alignment(Align::Stretch)
                    .child(Text::new("Inbox"))
                    .child(Divider::new())
                    .child(Text::new("Archive"))
                    .child(Divider::new())
                    .child(Text::new("Trash")),
            ),
        )),
        // Indents line a rule up with the text of a list rather than with its edge.
        Box::new(section(
            "Indented, thicker, coloured",
            Container::new().width(300.0).child(
                Column::new()
                    .cross_axis_alignment(Align::Stretch)
                    .child(Divider::new().indent(56.0))
                    .child(Divider::new().indent(16.0).end_indent(16.0))
                    .child(
                        Divider::new()
                            .height(24.0)
                            .thickness(4.0)
                            .radius(2.0)
                            .color(Color::hex("#6366F1")),
                    ),
            ),
        )),
        // A vertical divider separates items in a row; the row needs a height for the
        // rule to run down.
        Box::new(section(
            "VerticalDivider",
            Container::new().height(40.0).child(
                Row::new()
                    .cross_axis_alignment(Align::Stretch)
                    .child(Text::new("Bold"))
                    .child(VerticalDivider::new())
                    .child(Text::new("Italic"))
                    .child(VerticalDivider::new().width(24.0).thickness(2.0))
                    .child(Text::new("Underline"))
                    .child(VerticalDivider::new().indent(8.0).end_indent(8.0))
                    .child(Text::new("Strike")),
            ),
        )),
    ])
}
