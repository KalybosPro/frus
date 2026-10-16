//! `Card` — a themed surface that groups one piece of content: elevated, filled or
//! outlined.

use frus::{
    Align, Button, Card, CardVariant, Column, FontWeight, Icon, Icons, Row, Text, Variant, Widget,
    Wrap,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        // The three variants say how far the card stands off what is behind it: a shadow
        // (elevated, the default), a tonal fill, or only an outline.
        Box::new(section(
            "Variants",
            Wrap::new()
                .gap(8.0)
                .run_gap(8.0)
                .child(labelled(Card::new(), "Elevated"))
                .child(labelled(Card::new().filled(), "Filled"))
                .child(labelled(Card::new().outlined(), "Outlined")),
        )),
        // Every default is overridable: depth, rounding, colour, padding, margin.
        Box::new(section(
            "Custom elevation, radius and padding",
            Card::new()
                .variant(CardVariant::Elevated)
                .elevation(6.0)
                .radius(20.0)
                .padding(24.0)
                .child(Text::new("Further off the surface, rounder corners.")),
        )),
        // The usual shape of a card: a heading, some text, and actions at the bottom.
        Box::new(section(
            "Content and actions",
            Card::new().outlined().child(
                Column::new()
                    .cross_axis_alignment(Align::Start)
                    .spacing(8.0)
                    .child(
                        Row::new()
                            .spacing(8.0)
                            .child(Icon::new(Icons::LOCATION_ON))
                            .child(Text::new("Lisbon").size(18.0).weight(FontWeight::SemiBold)),
                    )
                    .child(Text::new(
                        "Three nights, two museums and far too many pastries.",
                    ))
                    .child(
                        Row::new()
                            .spacing(8.0)
                            .child(Button::new("Share").variant(Variant::Text))
                            .child(Button::new("Book").variant(Variant::Tonal)),
                    ),
            ),
        )),
    ])
}

/// A small card holding one word — the variants side by side.
fn labelled(card: Card, label: &str) -> Card {
    card.child(Text::new(label))
}
