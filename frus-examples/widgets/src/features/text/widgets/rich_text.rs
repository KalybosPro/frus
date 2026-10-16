//! `RichText` and `TextSpan` — one paragraph in several styles: a bold word, a link-
//! coloured phrase, a price in two sizes.
//!
//! A `TextSpan` is a tree: each span has its own text and children, and a child
//! inherits whatever its parent's style says unless it says otherwise.

use frus::{
    Color, Container, FontFamily, FontWeight, RichText, TextAlign, TextDecoration, TextOverflow,
    TextSpan, TextStyle, Widget,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        // The basics: plain text, then children that change one thing each.
        Box::new(section(
            "Mixed styles in one line",
            RichText::new(
                TextSpan::new("frus is ")
                    .child(TextSpan::new("fast").bold())
                    .child(TextSpan::new(", "))
                    .child(TextSpan::new("portable").italic())
                    .child(TextSpan::new(" and "))
                    .child(
                        TextSpan::new("yours")
                            .underline()
                            .color(Color::hex("#4F46E5")),
                    )
                    .child(TextSpan::new(".")),
            ),
        )),
        // Inheritance: the outer span sets size and colour once; the children only say
        // what differs.
        Box::new(section(
            "Inherited style",
            RichText::new(
                TextSpan::new("Total: ")
                    .style(TextStyle::new(16.0).color(Color::hex("#475569")))
                    .child(
                        TextSpan::new("€42")
                            .size(32.0)
                            .weight(FontWeight::Bold)
                            .color(Color::hex("#0F172A")),
                    )
                    .child(TextSpan::new(".99").size(18.0).color(Color::hex("#0F172A")))
                    .child(TextSpan::new(" / month")),
            ),
        )),
        // A whole span can carry a full `TextStyle` — here monospace for inline code —
        // and a decoration in a colour of its own.
        Box::new(section(
            "Inline code and a coloured strikethrough",
            RichText::new(
                TextSpan::new("Call ")
                    .child(
                        TextSpan::new("Text::new()")
                            .style(TextStyle::NONE.family(FontFamily::Monospace))
                            .color(Color::hex("#BE185D")),
                    )
                    .child(TextSpan::new(" instead of "))
                    .child(
                        TextSpan::new("label()")
                            .decoration(TextDecoration::STRIKETHROUGH)
                            .decoration_color(Color::hex("#DC2626")),
                    )
                    .child(TextSpan::new(".")),
            ),
        )),
        // A paragraph wraps, aligns and truncates like a `Text` does; the base style is
        // what every span starts from.
        Box::new(section(
            "A wrapped, centred paragraph",
            Container::new().width(300.0).child(
                RichText::new(
                    TextSpan::new("Rich text ")
                        .child(TextSpan::new("wraps").bold())
                        .child(TextSpan::new(
                            " at the width it is given, keeps each span's style across the \
                             line breaks, and can be ",
                        ))
                        .child(TextSpan::new("centred").italic())
                        .child(TextSpan::new(" or cut short after a number of lines.")),
                )
                .base_style(TextStyle::new(15.0).height(1.4))
                .align(TextAlign::Center)
                .max_lines(3)
                .overflow(TextOverflow::Ellipsis),
            ),
        )),
    ])
}
