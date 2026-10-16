//! `Padding` — empty space around a child, and nothing else.

use frus::{Color, Container, Insets, InsetsDirectional, Padding, Text, Widget};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        // A single number pads every side by the same amount.
        Box::new(section(
            "The same on every side",
            framed(Padding::new(24.0, label("24 px all round"))),
        )),
        // `Insets::symmetric(horizontal, vertical)`: wide and short, the shape of a button.
        Box::new(section(
            "Symmetric",
            framed(Padding::new(
                Insets::symmetric(32.0, 8.0),
                label("32 across, 8 down"),
            )),
        )),
        // `Insets::new(top, right, bottom, left)` sets each side on its own.
        Box::new(section(
            "Each side",
            framed(Padding::new(
                Insets::new(4.0, 8.0, 24.0, 48.0),
                label("4 / 8 / 24 / 48"),
            )),
        )),
        // Directional insets say "start" and "end" instead of left and right, so the
        // indent follows the reading direction: on the right in Arabic or Hebrew.
        Box::new(section(
            "Start and end",
            framed(Padding::new(
                InsetsDirectional::only_start(40.0),
                label("Indented at the start"),
            )),
        )),
    ])
}

/// The tinted box shows where the padding is: the label sits in a white box, the tint
/// around it is the padding.
fn framed(padding: Padding) -> Container {
    Container::new().color(Color::hex("#C7D2FE")).child(padding)
}

fn label(text: &str) -> Container {
    Container::new()
        .padding(8.0)
        .color(Color::hex("#FFFFFF"))
        .child(Text::new(text).color(Color::hex("#312E81")))
}
