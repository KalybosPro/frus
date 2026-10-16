//! `Text` — a run of text in one style: size, weight, colour, slant, decoration,
//! alignment, wrapping and line limits are all set on the widget.

use frus::{
    Align, Color, Column, Container, FontWeight, Row, Text, TextAlign, TextDecoration,
    TextOverflow, Widget, Wrap,
};

use crate::common::shared::{section, sections};

const LONG: &str = "Text wraps at the width it is given. This sentence is long enough to need \
                    several lines in a narrow box, which is where alignment, line limits and \
                    ellipses start to matter.";

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        // Unstyled, a text takes the theme's body style and colour, so it reads right in
        // light and dark mode without any setting.
        Box::new(section("Default", Text::new("Hello, frus!"))),
        // Sizes are logical pixels; they grow with the reader's system font size.
        Box::new(section(
            "Sizes",
            Wrap::new()
                .gap(16.0)
                .run_gap(8.0)
                .align(Align::Baseline)
                .child(Text::new("12 px").size(12.0))
                .child(Text::new("16 px").size(16.0))
                .child(Text::new("24 px").size(24.0))
                .child(Text::new("32 px").size(32.0)),
        )),
        Box::new(section(
            "Weights and italic",
            Wrap::new()
                .gap(16.0)
                .run_gap(8.0)
                .child(Text::new("Regular").weight(FontWeight::Regular))
                .child(Text::new("Medium").weight(FontWeight::Medium))
                .child(Text::new("SemiBold").weight(FontWeight::SemiBold))
                .child(Text::new("Bold").weight(FontWeight::Bold))
                .child(Text::new("Italic").italic()),
        )),
        // A fixed colour does not follow the theme: keep it for meaning (errors,
        // success), and let ordinary text inherit.
        Box::new(section(
            "Colours",
            Wrap::new()
                .gap(16.0)
                .run_gap(8.0)
                .child(Text::new("Indigo").color(Color::hex("#4F46E5")))
                .child(Text::new("Error").color(Color::hex("#DC2626")))
                .child(Text::new("Success").color(Color::hex("#16A34A"))),
        )),
        // Lines under, through or over the text; the line can have its own colour.
        Box::new(section(
            "Decoration",
            Wrap::new()
                .gap(16.0)
                .run_gap(8.0)
                .child(Text::new("Underline").underline())
                .child(Text::new("Strikethrough").strikethrough())
                .child(Text::new("Overline").decoration(TextDecoration::OVERLINE))
                .child(
                    Text::new("Red underline")
                        .underline()
                        .decoration_color(Color::hex("#DC2626")),
                ),
        )),
        // Alignment places the lines inside the text's box, so it shows when the box is
        // wider than the text: here, 300 px.
        Box::new(section(
            "Alignment",
            Column::new()
                .spacing(6.0)
                .child(boxed(Text::new("Start").align(TextAlign::Start)))
                .child(boxed(Text::new("Center").align(TextAlign::Center)))
                .child(boxed(Text::new("End").align(TextAlign::End)))
                .child(boxed(Text::new(LONG).align(TextAlign::Justify))),
        )),
        // A line limit, and what happens to the rest: an ellipsis, a fade, or a hard cut.
        Box::new(section(
            "Line limits",
            Column::new()
                .spacing(8.0)
                .child(boxed(Text::new(LONG).max_lines(2).ellipsis()))
                .child(boxed(
                    Text::new(LONG).max_lines(1).overflow(TextOverflow::Fade),
                ))
                .child(boxed(
                    Text::new(LONG).no_wrap().overflow(TextOverflow::Clip),
                )),
        )),
        // `height` sets the line height as a multiple of the size: 1.6 airs a paragraph
        // out, 1.0 packs it.
        Box::new(section(
            "Line height",
            Row::new()
                .spacing(12.0)
                .cross_axis_alignment(Align::Start)
                .child(
                    Container::new()
                        .width(150.0)
                        .child(Text::new(LONG).size(13.0).height(1.0)),
                )
                .child(
                    Container::new()
                        .width(150.0)
                        .child(Text::new(LONG).size(13.0).height(1.6)),
                ),
        )),
    ])
}

/// A 300 px box with a faint outline, so you can see the text's room.
fn boxed(text: Text) -> Container {
    Container::new()
        .width(300.0)
        .padding(6.0)
        .border(1.0, Color::hex("#CBD5E1"))
        .child(text)
}
