//! `Row` and `Column` — children in a line, across or down: the two containers most
//! screens are built from.

use frus::{
    Align, Alignment, Color, Column, Container, Justify, MainAxisSize, Row, Text, TextDirection,
    VerticalDirection, Widget,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        // `spacing` puts the same gap between every pair of children — no spacer widgets.
        Box::new(section(
            "Row with spacing",
            Row::new()
                .spacing(8.0)
                .child(block("A", 48.0))
                .child(block("B", 48.0))
                .child(block("C", 48.0)),
        )),
        // A row fills the width it is given, so the main-axis alignment decides where the
        // free room goes: at the end, in the middle, or shared between the children.
        Box::new(section(
            "Main axis alignment",
            Column::new()
                .spacing(8.0)
                .child(lane(Justify::Start, "Start"))
                .child(lane(Justify::Center, "Center"))
                .child(lane(Justify::End, "End"))
                .child(lane(Justify::SpaceBetween, "SpaceBetween"))
                .child(lane(Justify::SpaceEvenly, "SpaceEvenly")),
        )),
        // The cross axis is the other direction. Children of different heights line up on
        // their tops, their centres (the default) or their bottoms, or all stretch to the
        // tallest one.
        Box::new(section(
            "Cross axis alignment",
            Row::new()
                .spacing(16.0)
                .child(tall_row(Align::Start, "Start"))
                .child(tall_row(Align::Center, "Center"))
                .child(tall_row(Align::End, "End"))
                .child(tall_row(Align::Stretch, "Stretch")),
        )),
        // `MainAxisSize::Min` makes the row only as wide as its children — the size you
        // want for a row that is itself placed by something else, such as a button's
        // icon and label.
        Box::new(section(
            "MainAxisSize::Min",
            Container::new().color(Color::hex("#E0E7FF")).child(
                Row::new()
                    .main_axis_size(MainAxisSize::Min)
                    .spacing(8.0)
                    .child(block("Shrink", 70.0))
                    .child(block("wrapped", 70.0)),
            ),
        )),
        // A column is the same thing downwards. `vertical_direction(Up)` stacks from the
        // bottom; a row's `text_direction(Rtl)` lays its children out right to left.
        Box::new(section(
            "Column, and both directions reversed",
            Row::new()
                .spacing(24.0)
                .cross_axis_alignment(Align::Start)
                .child(
                    Column::new()
                        .spacing(4.0)
                        .child(block("1", 48.0))
                        .child(block("2", 48.0))
                        .child(block("3", 48.0)),
                )
                .child(
                    Column::new()
                        .spacing(4.0)
                        .vertical_direction(VerticalDirection::Up)
                        .child(block("1", 48.0))
                        .child(block("2", 48.0))
                        .child(block("3", 48.0)),
                )
                .child(
                    Row::new()
                        .main_axis_size(MainAxisSize::Min)
                        .spacing(4.0)
                        .text_direction(TextDirection::Rtl)
                        .child(block("1", 40.0))
                        .child(block("2", 40.0))
                        .child(block("3", 40.0)),
                ),
        )),
    ])
}

/// A full-width row of three blocks, aligned along its length by `justify`.
fn lane(justify: Justify, name: &str) -> Container {
    Container::new().color(Color::hex("#EEF2FF")).child(
        Row::new()
            .main_axis_alignment(justify)
            .spacing(4.0)
            .child(block(name, 110.0))
            .child(block("·", 32.0))
            .child(block("·", 32.0)),
    )
}

/// Three children of different heights, aligned across the row by `align`.
fn tall_row(align: Align, name: &str) -> Column {
    // Stretch only stretches a child that has no height of its own.
    let bar = |height: f32, color: &str| {
        let bar = Container::new().width(16.0).color(Color::hex(color));
        if align == Align::Stretch {
            bar
        } else {
            bar.height(height)
        }
    };
    Column::new()
        .spacing(4.0)
        .child(Text::new(name).size(12.0))
        .child(
            Container::new()
                .height(80.0)
                .color(Color::hex("#EEF2FF"))
                .child(
                    Row::new()
                        .main_axis_size(MainAxisSize::Min)
                        .cross_axis_alignment(align)
                        .spacing(4.0)
                        .child(bar(24.0, "#6366F1"))
                        .child(bar(48.0, "#4F46E5"))
                        .child(bar(36.0, "#818CF8")),
                ),
        )
}

fn block(label: &str, width: f32) -> Container {
    Container::new()
        .width(width)
        .height(36.0)
        .radius(6.0)
        .color(Color::hex("#6366F1"))
        .alignment(Alignment::CENTER)
        .child(Text::new(label).size(13.0).color(Color::hex("#FFFFFF")))
}
