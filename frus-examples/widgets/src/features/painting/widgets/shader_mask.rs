//! `ShaderMask` — lays a gradient over a child's own pixels: a headline painted in a
//! gradient, a list that fades out at its edge, a spotlight.
//!
//! By default the mask's alpha multiplies the child (a fade). With `BlendMode::SrcIn`
//! the mask's colours replace the child's wherever the child is drawn — a gradient
//! fill for text or icons.

use frus::{Align, Alignment, BlendMode, Color, Column, Container, ShaderMask, Text, Widget};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        // `linear(from, to, from_colour, to_colour)`, points as fractions of the box.
        Box::new(section(
            "Gradient text",
            ShaderMask::linear(
                (0.0, 0.0),
                (1.0, 0.0),
                Color::hex("#F59E0B"),
                Color::hex("#DB2777"),
            )
            .blend(BlendMode::SrcIn)
            .child(Text::new("Gradient headline").size(36.0)),
        )),
        // The default blend multiplies alpha in: opaque at the top, gone at the bottom —
        // the edge of a list that continues past view.
        Box::new(section(
            "Fade out at the bottom",
            ShaderMask::fade_out_bottom().child(
                (1..=6).fold(
                    Column::new()
                        .spacing(4.0)
                        .cross_axis_alignment(Align::Start),
                    |column, i| {
                        column.child(
                            Container::new()
                                .width(240.0)
                                .padding(8.0)
                                .color(Color::hex("#E0E7FF"))
                                .child(Text::new(format!("Row {i}")).color(Color::hex("#312E81"))),
                        )
                    },
                ),
            ),
        )),
        // A radial mask: visible in the middle, fading towards the edges — a spotlight
        // or a vignette. The default blend multiplies colour as well as alpha, so a pure
        // fade is drawn in white, from opaque to transparent.
        Box::new(section(
            "Radial spotlight",
            ShaderMask::radial(
                (0.5, 0.5),
                0.6,
                Color::rgba(1.0, 1.0, 1.0, 1.0),
                Color::rgba(1.0, 1.0, 1.0, 0.0),
            )
            .child(
                Container::new()
                    .width(240.0)
                    .height(120.0)
                    .color(Color::hex("#0EA5E9"))
                    .alignment(Alignment::CENTER)
                    .child(Text::new("In the spotlight").color(Color::hex("#FFFFFF"))),
            ),
        )),
    ])
}
