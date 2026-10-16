//! `LayoutBuilder` and `OrientationBuilder` — build from the room actually given, not
//! from the window's size, so a component adapts wherever it is placed.
//!
//! The content is built on the fly every frame from the box: it keeps no state of its
//! own, so keep the closure cheap and put any state outside it.

use frus::{
    Align, Alignment, Color, Column, Container, Expanded, LayoutBuilder, Orientation,
    OrientationBuilder, Row, SizedBox, Text, Widget,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        // The closure receives the size of its box. Resize the window to see the number
        // (and the layout) change.
        Box::new(section(
            "The box it was given",
            LayoutBuilder::new(|size| {
                Container::new()
                    .padding(12.0)
                    .color(Color::hex("#E0E7FF"))
                    .child(Text::new(format!("{:.0} px wide", size.width)))
            }),
        )),
        // The common use: one arrangement when there is room, another when there is not.
        // The same card, at two widths.
        Box::new(section(
            "Side by side when it fits",
            Column::new()
                .cross_axis_alignment(Align::Start)
                .spacing(12.0)
                .child(SizedBox::new(card()).width(320.0))
                .child(SizedBox::new(card()).width(200.0)),
        )),
        // `OrientationBuilder` answers the simpler question: is the box wider than tall?
        // Here it is asked about two fixed boxes.
        Box::new(section(
            "OrientationBuilder",
            Row::new()
                .spacing(12.0)
                .child(SizedBox::new(oriented()).width(160.0).height(90.0))
                .child(SizedBox::new(oriented()).width(90.0).height(160.0)),
        )),
    ])
}

/// A picture and a text: in a row from 260 px up, stacked below that.
fn card() -> LayoutBuilder {
    LayoutBuilder::new(|size| -> Box<dyn Widget> {
        let picture = Container::new()
            .width(80.0)
            .height(60.0)
            .radius(6.0)
            .color(Color::hex("#6366F1"));
        let caption =
            Text::new(format!("{:.0} px: ", size.width) + "a caption beside or under the picture")
                .wrap();
        if size.width >= 260.0 {
            Box::new(
                Row::new()
                    .spacing(12.0)
                    .child(picture)
                    .child(Expanded::new(caption)),
            )
        } else {
            Box::new(
                Column::new()
                    .cross_axis_alignment(Align::Start)
                    .spacing(8.0)
                    .child(picture)
                    .child(caption),
            )
        }
    })
}

fn oriented() -> LayoutBuilder {
    OrientationBuilder::new(|orientation| {
        let (label, color) = match orientation {
            Orientation::Landscape => ("Landscape", "#10B981"),
            Orientation::Portrait => ("Portrait", "#F59E0B"),
        };
        Container::new()
            .color(Color::hex(color))
            .alignment(Alignment::CENTER)
            .child(Text::new(label).color(Color::hex("#FFFFFF")))
    })
}
