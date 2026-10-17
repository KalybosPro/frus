//! `TextStyle` — a style as a value: define it once, share it, merge it, and hand it to
//! `Text::styled`, a `TextSpan` or a whole subtree with `DefaultTextStyle`.
//!
//! Every field of a style is optional. A style that only says "size 20" leaves the
//! weight and colour to whatever is around it — which is what makes styles layer.

use frus::{
    Align, Color, Column, DefaultTextStyle, FontWeight, Text, TextAlign, TextStyle, Widget,
};

use crate::common::shared::{section, sections};

/// A style shared by several texts: a `const`, since every builder is `const fn`.
const LABEL: TextStyle = TextStyle::new(12.0)
    .weight(FontWeight::SemiBold)
    .letter_spacing(1.2)
    .color(Color::rgb(0.39, 0.40, 0.95));

pub fn example() -> Box<dyn Widget> {
    sections(vec![
        // `Text::styled` takes a whole style at once.
        Box::new(section(
            "A shared style",
            Column::new()
                .cross_axis_alignment(Align::Start)
                .spacing(4.0)
                .child(Text::styled("SECTION ONE", LABEL))
                .child(Text::styled("SECTION TWO", LABEL)),
        )),
        // `merge` lays one style over another: what the second says wins, what it leaves
        // open is kept. Here, the label made bigger and red.
        Box::new(section(
            "Merging",
            Text::styled(
                "OVERRIDDEN",
                LABEL.merge(TextStyle::new(18.0).color(Color::hex("#DC2626"))),
            ),
        )),
        // Letter spacing and line height are part of the style.
        Box::new(section(
            "Letter spacing",
            Column::new()
                .cross_axis_alignment(Align::Start)
                .spacing(4.0)
                .child(Text::styled(
                    "tight",
                    TextStyle::new(18.0).letter_spacing(-1.0),
                ))
                .child(Text::styled("normal", TextStyle::new(18.0)))
                .child(Text::styled(
                    "wide",
                    TextStyle::new(18.0).letter_spacing(4.0),
                )),
        )),
        // `DefaultTextStyle::around` gives every text in a subtree a default — the way a
        // card or a toolbar styles all its text at once. A text's own settings still win.
        Box::new(section(
            "DefaultTextStyle for a subtree",
            DefaultTextStyle {
                style: TextStyle::new(15.0).italic().color(Color::hex("#0F766E")),
                align: Some(TextAlign::Start),
                ..DefaultTextStyle::NONE
            }
            .around(
                Column::new()
                    .cross_axis_alignment(Align::Start)
                    .spacing(4.0)
                    .child(Text::new("Inherits the italic teal default."))
                    .child(Text::new("So does this one."))
                    .child(Text::new("This one asks for bold.").weight(FontWeight::Bold)),
            ),
        )),
    ])
}
