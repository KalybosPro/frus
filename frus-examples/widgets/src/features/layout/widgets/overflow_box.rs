//! `UnconstrainedBox`, `OverflowBox`, `SizedOverflowBox` and `ConstraintsTransformBox` —
//! let a child be a size its parent did not offer.

use frus::{
    Alignment, AxisConstraint, Color, ConstraintsTransformBox, Container, MainAxisSize,
    OverflowBox, Row, SizedOverflowBox, Text, UnconstrainedBox, Widget, Wrap,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        // A fixed-size container stretches its child to fill it (left). Wrapped in an
        // `UnconstrainedBox`, the child keeps its natural size instead (right).
        Box::new(section(
            "UnconstrainedBox",
            Wrap::new()
                .gap(12.0)
                .run_gap(12.0)
                .child(frame(tag("stretched")))
                .child(frame(UnconstrainedBox::new(tag("natural size")))),
        )),
        // `OverflowBox` lays its child out at a size of its own and lets it spill past
        // the box without moving anything around it. It does not clip.
        Box::new(section(
            "OverflowBox",
            Container::new().padding(24.0).child(
                Container::new()
                    .width(80.0)
                    .height(40.0)
                    .color(Color::hex("#C7D2FE"))
                    .child(
                        OverflowBox::new(
                            Container::new()
                                .radius(8.0)
                                .border(2.0, Color::hex("#4F46E5")),
                        )
                        .width(120.0)
                        .height(72.0),
                    ),
            ),
        )),
        // `SizedOverflowBox` takes a stated size in the layout (a 24 px slot here) and
        // shows a bigger child in it, anchored by its alignment.
        Box::new(section(
            "SizedOverflowBox",
            Container::new().padding(16.0).child(
                Row::new()
                    .main_axis_size(MainAxisSize::Min)
                    .child(tag("before"))
                    .child(
                        SizedOverflowBox::new(
                            24.0,
                            24.0,
                            Container::new()
                                .width(40.0)
                                .height(40.0)
                                .radius(20.0)
                                .color(Color::hex("#F59E0B")),
                        )
                        .alignment(Alignment::CENTER),
                    )
                    .child(tag("after")),
            ),
        )),
        // The general form: say per axis what the child is given. Here the child gets a
        // fixed 140 px across, whatever the page offers, and its natural height down.
        Box::new(section(
            "ConstraintsTransformBox",
            ConstraintsTransformBox::new(
                Container::new()
                    .padding(8.0)
                    .color(Color::hex("#E0E7FF"))
                    .child(Text::new("Laid out at exactly 140 px wide, wrapping.").wrap()),
            )
            .width(AxisConstraint::Fixed(140.0))
            .height(AxisConstraint::Unbounded)
            .alignment(Alignment::TOP_LEFT),
        )),
    ])
}

fn frame(child: impl Widget + 'static) -> Container {
    Container::new()
        .width(160.0)
        .height(72.0)
        .border(1.0, Color::hex("#A5B4FC"))
        .child(child)
}

fn tag(label: &str) -> Container {
    Container::new()
        .padding(8.0)
        .color(Color::hex("#6366F1"))
        .child(Text::new(label).size(13.0).color(Color::hex("#FFFFFF")))
}
