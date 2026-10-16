//! `AnimatedScale`, `AnimatedRotation` and `AnimatedSlide` — grow, turn or shift a child to
//! each new value.
//!
//! All three only change how the child is **painted**: its box stays where the layout put
//! it, so the neighbours never move.

use std::f32::consts::PI;

use frus::{
    component, Align, Alignment, AnimatedRotation, AnimatedScale, AnimatedSlide, Button, Column,
    Component, Container, Curve, Icon, Icons, Row, Text, Variant, Widget,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![Box::new(section(
        "Scale, rotation and slide on one flag",
        transforms(),
    ))])
}

fn transforms() -> Component {
    component(|cx| {
        let active = cx.use_state(|| false);
        let on = active.get();
        let scheme = &cx.theme().scheme;

        let tile = |icon| {
            Container::new()
                .width(64.0)
                .height(64.0)
                .radius(12.0)
                .color(scheme.primary_container)
                .alignment(Alignment::CENTER)
                .child(Icon::new(icon).color(scheme.on_primary_container))
        };
        let labelled = |label: &str, child: Box<dyn Widget>| {
            Column::new()
                .spacing(8.0)
                .cross_axis_alignment(Align::Center)
                .child_boxed(child)
                .child(Text::new(label).size(12.0))
        };

        let toggle = active.clone();
        Box::new(
            Column::new()
                .spacing(20.0)
                .cross_axis_alignment(Align::Start)
                .child(
                    Button::new(if on { "Reset" } else { "Animate" })
                        .variant(Variant::Tonal)
                        .on_press(move || toggle.update(|on| *on = !*on)),
                )
                .child(
                    Row::new()
                        .spacing(32.0)
                        // A press feedback, a favourite that pops: scale about the centre.
                        .child(labelled(
                            "AnimatedScale",
                            Box::new(AnimatedScale::new(
                                if on { 1.3 } else { 1.0 },
                                0.25,
                                Curve::ease_out(),
                                tile(Icons::FAVORITE),
                            )),
                        ))
                        // A refresh arrow, an expand chevron: the angle is in radians.
                        .child(labelled(
                            "AnimatedRotation",
                            Box::new(AnimatedRotation::new(
                                if on { PI } else { 0.0 },
                                0.4,
                                Curve::ease_in_out(),
                                tile(Icons::REFRESH),
                            )),
                        ))
                        // The offset is a fraction of the child's own size: 0.5 is half
                        // its width.
                        .child(labelled(
                            "AnimatedSlide",
                            Box::new(AnimatedSlide::new(
                                if on { 0.5 } else { 0.0 },
                                0.0,
                                0.3,
                                Curve::ease_out(),
                                tile(Icons::SEND),
                            )),
                        )),
                ),
        )
    })
}
