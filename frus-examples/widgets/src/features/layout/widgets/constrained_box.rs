//! `ConstrainedBox`, `LimitedBox`, `IntrinsicWidth` and `IntrinsicHeight` — change how
//! big a child is allowed to be, rather than setting its size outright.

use frus::{
    Align, Color, ColoredBox, Column, ConstrainedBox, Container, IntrinsicHeight, IntrinsicWidth,
    LimitedBox, Padding, Row, SizedBox, Text, Widget,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        // A floor: the box is at least 120 px wide however short its label, the way
        // buttons keep a minimum width. The tint shows the box.
        Box::new(section(
            "Minimum width",
            Row::new()
                .spacing(8.0)
                .child(floor("OK"))
                .child(floor("A longer label")),
        )),
        // A ceiling: a paragraph never gets wider than 240 px, however wide the page — a
        // comfortable reading measure.
        Box::new(section(
            "Maximum width",
            ConstrainedBox::new(
                Text::new(
                    "This paragraph is capped at 240 px wide, so its lines stay short enough \
                     to read comfortably even on a wide window.",
                )
                .wrap(),
            )
            .max_width(240.0),
        )),
        // `tight` makes the box exactly that size, whatever the child would like.
        Box::new(section(
            "Tight",
            ConstrainedBox::tight(180.0, 48.0, pill("exactly 180 × 48")),
        )),
        // `LimitedBox` only caps an axis that has no end. This page scrolls, so its height
        // has none: a box that asks for all the room is held to 80 px. Where the room is
        // bounded the limit does nothing.
        Box::new(section(
            "LimitedBox in a scrolling page",
            LimitedBox::new(SizedBox::expand(
                Container::new()
                    .color(Color::hex("#C7D2FE"))
                    .child(Text::new("held to 80 px").color(Color::hex("#312E81"))),
            ))
            .max_height(80.0),
        )),
        // `IntrinsicWidth` sizes a column to its widest child, and the stretched children
        // all take that width: a menu whose items are all as wide as the longest one.
        Box::new(section(
            "IntrinsicWidth",
            IntrinsicWidth::width(
                Column::new()
                    .spacing(4.0)
                    .cross_axis_alignment(Align::Stretch)
                    .child(pill("Open"))
                    .child(pill("Save as…"))
                    .child(pill("Export to PDF")),
            ),
        )),
        // `IntrinsicHeight` does the same downwards: the side bar matches the height of
        // the text beside it, whatever that turns out to be.
        Box::new(section(
            "IntrinsicHeight",
            IntrinsicHeight::height(
                Row::new()
                    .spacing(8.0)
                    .cross_axis_alignment(Align::Stretch)
                    .child(Container::new().width(4.0).color(Color::hex("#6366F1")))
                    .child(
                        Text::new(
                            "A quoted line,\nas tall as it needs to be,\nwith a bar beside it.",
                        )
                        .wrap(),
                    ),
            ),
        )),
    ])
}

fn pill(label: &str) -> Container {
    Container::new()
        .padding(10.0)
        .radius(6.0)
        .color(Color::hex("#6366F1"))
        .child(Text::new(label).size(13.0).color(Color::hex("#FFFFFF")))
}

/// A label in a box with a 120 px floor; the tint is the box, not the label.
fn floor(label: &str) -> ColoredBox {
    ColoredBox::new(
        Color::hex("#C7D2FE"),
        ConstrainedBox::new(Padding::new(
            10.0,
            Text::new(label).size(13.0).color(Color::hex("#312E81")),
        ))
        .min_width(120.0),
    )
}
