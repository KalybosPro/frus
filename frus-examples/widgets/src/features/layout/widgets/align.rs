//! `Center` and `Aligned` — place one child somewhere inside the room it is given.

use frus::{
    Aligned, Alignment, AlignmentDirectional, Center, Color, Container, Text, Widget, Wrap,
};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    sections(vec![
        // `Center` takes all the room it is offered and puts its child in the middle.
        Box::new(section("Center", frame(Center::new(dot("center"))))),
        // `Aligned` does the same at any of the nine named spots — or anywhere between
        // them: `Alignment::new(x, y)` runs from -1 (left/top) to 1 (right/bottom).
        Box::new(section(
            "Aligned",
            Wrap::new()
                .gap(12.0)
                .run_gap(12.0)
                .child(frame(Aligned::new(Alignment::TOP_LEFT, dot("top left"))))
                .child(frame(Aligned::new(
                    Alignment::BOTTOM_RIGHT,
                    dot("bottom right"),
                )))
                .child(frame(Aligned::new(
                    Alignment::new(0.5, -0.5),
                    dot("(0.5, -0.5)"),
                ))),
        )),
        // Directional alignment says "start" and "end": the child moves to the right in a
        // right-to-left language without any change to the code.
        Box::new(section(
            "Directional",
            frame(Aligned::new(
                AlignmentDirectional::CENTER_END,
                dot("center end"),
            )),
        )),
        // A size factor makes the box a multiple of its child instead of taking the whole
        // room: here twice as wide and three times as tall as the label.
        Box::new(section(
            "Size factors",
            Container::new().color(Color::hex("#E0E7FF")).child(
                Center::new(dot("2× wide, 3× tall"))
                    .width_factor(2.0)
                    .height_factor(3.0),
            ),
        )),
    ])
}

/// A fixed box, so you can see where inside it the child ends up.
fn frame(child: impl Widget + 'static) -> Container {
    Container::new()
        .width(200.0)
        .height(120.0)
        .radius(8.0)
        .color(Color::hex("#E0E7FF"))
        .child(child)
}

fn dot(text: &str) -> Container {
    Container::new()
        .padding(6.0)
        .radius(6.0)
        .color(Color::hex("#4F46E5"))
        .child(Text::new(text).size(12.0).color(Color::hex("#FFFFFF")))
}
