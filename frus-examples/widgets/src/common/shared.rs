//! Pieces the demos share: a titled section, so one demo can show a widget in several
//! configurations, one under the other.

use frus::{Align, Column, FontWeight, Text, Widget};

/// A small heading over one configuration of a widget.
///
/// ```ignore
/// Column::new()
///     .child(section("Filled", Button::new("Save")))
///     .child(section("Outlined", Button::new("Cancel").variant(Variant::Outlined)))
/// ```
pub fn section(title: &str, child: impl Widget + 'static) -> Column {
    Column::new()
        .spacing(8.0)
        .cross_axis_alignment(Align::Start)
        .child(Text::new(title).size(13.0).weight(FontWeight::SemiBold))
        .child(child)
}

/// Several sections, one under the other, with room between them — the usual body of a
/// demo.
pub fn sections(children: Vec<Box<dyn Widget>>) -> Box<dyn Widget> {
    let column = children.into_iter().fold(
        Column::new()
            .spacing(24.0)
            .cross_axis_alignment(Align::Start),
        |column, child| column.child(child),
    );
    Box::new(column)
}
