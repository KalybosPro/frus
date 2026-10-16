//! `AnimatedAlign` — moves a child to a new anchor inside its box instead of letting it
//! jump: a badge changing corner, a thumb crossing a track.

use frus::{
    component, Align, Alignment, AnimatedAlign, Button, Column, Component, Container, Curve, Row,
    Switch, Text, Variant, Widget,
};

use crate::common::shared::{section, sections};

/// The four corners the dot visits, in order.
const CORNERS: [(Alignment, &str); 4] = [
    (Alignment::TOP_LEFT, "top left"),
    (Alignment::TOP_RIGHT, "top right"),
    (Alignment::BOTTOM_RIGHT, "bottom right"),
    (Alignment::BOTTOM_LEFT, "bottom left"),
];

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        // The anchor is the animated quantity: change it and the child slides there.
        Box::new(section("Corner to corner", corners())),
        // The same idea, small: a home-made toggle whose knob glides across.
        Box::new(section("A sliding knob", knob())),
    ])
}

fn corners() -> Component {
    component(|cx| {
        let corner = cx.use_state(|| 0_usize);
        let (alignment, name) = CORNERS[corner.get()];
        let scheme = &cx.theme().scheme;

        let next = corner.clone();
        Box::new(
            Column::new()
                .spacing(12.0)
                .cross_axis_alignment(Align::Start)
                .child(
                    Button::new("Next corner")
                        .variant(Variant::Tonal)
                        .on_press(move || next.update(|i| *i = (*i + 1) % CORNERS.len())),
                )
                .child(
                    Container::new()
                        .width(280.0)
                        .height(140.0)
                        .padding(8.0)
                        .radius(12.0)
                        .color(scheme.surface_container_high)
                        .child(AnimatedAlign::new(
                            alignment,
                            0.5,
                            Curve::ease_in_out(),
                            Container::new()
                                .width(36.0)
                                .height(36.0)
                                .radius(18.0)
                                .color(scheme.primary),
                        )),
                )
                .child(Text::new(format!("Anchored {name}")).size(13.0)),
        )
    })
}

fn knob() -> Component {
    component(|cx| {
        let on = cx.use_state(|| false);
        let is_on = on.get();
        let scheme = &cx.theme().scheme;

        // A framework `Switch` alongside, driven by the same state, for comparison.
        let flip = on.clone();
        let track = Container::new()
            .width(64.0)
            .height(32.0)
            .padding(4.0)
            .radius(16.0)
            .color(if is_on {
                scheme.primary
            } else {
                scheme.outline_variant
            })
            .on_click(move || flip.update(|on| *on = !*on))
            .child(AnimatedAlign::new(
                if is_on {
                    Alignment::CENTER_RIGHT
                } else {
                    Alignment::CENTER_LEFT
                },
                0.15,
                Curve::ease_out(),
                Container::new()
                    .width(24.0)
                    .height(24.0)
                    .radius(12.0)
                    .color(scheme.surface),
            ));

        let set = on.clone();
        Box::new(
            Row::new()
                .spacing(24.0)
                .cross_axis_alignment(Align::Center)
                .child(track)
                .child(Switch::new(is_on).on_toggle(move |value| set.set(value)))
                .child(Text::new(if is_on { "On" } else { "Off" })),
        )
    })
}
