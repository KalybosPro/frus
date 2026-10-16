//! `Tooltip` — a short label that appears while the pointer rests on a widget (or on a
//! long press). For naming what an icon does; never for something the user must read.

use frus::{Color, Icon, IconButton, Icons, Insets, Row, Text, TextStyle, Tooltip, Widget, Wrap};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        // Wrap any widget. (An `IconButton` already shows its `label` as a tooltip.)
        Box::new(section(
            "Hover these",
            Wrap::new()
                .gap(16.0)
                .run_gap(8.0)
                .child(Tooltip::new("Saved to your library").child(Icon::new(Icons::BOOKMARK)))
                .child(
                    Tooltip::new("Plain text has tooltips too").child(Text::new("What is this?")),
                )
                .child(IconButton::new(Icons::SHARE).label("Share this page")),
        )),
        // Bubble colour, text, padding, shape and width are all overridable. A long
        // message wraps at `max_width`.
        Box::new(section(
            "Custom look",
            Row::new().spacing(16.0).child(
                Tooltip::new(
                    "A longer explanation, wrapped at 200 pixels so the bubble stays readable.",
                )
                .background(Color::hex("#FEF3C7"))
                .text_style(TextStyle::new(13.0).color(Color::hex("#78350F")))
                .padding(Insets::symmetric(12.0, 8.0))
                .radius(8.0)
                .max_width(200.0)
                .child(Icon::new(Icons::INFO).color(Color::hex("#D97706"))),
            ),
        )),
    ])
}
