//! `AnimatedPositioned` — a `Stack` layer whose pins (left, top, right, bottom, width,
//! height) move to each new value: a panel that slides in, a card that travels.
//!
//! Only a pin set on **both** sides of a change travels: an edge that appears or disappears
//! is a different arrangement, and takes effect at once. So say where the layer is in both
//! states.

use frus::{
    component, Align, Alignment, AnimatedPositioned, Button, Column, Component, Container, Curve,
    Row, Stack, StackFit, Text, Variant, Widget,
};

use crate::common::shared::{section, sections};

const STAGE_HEIGHT: f32 = 180.0;
const PANEL_HEIGHT: f32 = 110.0;

pub fn example() -> Box<dyn Widget> {
    sections(vec![
        // A bottom panel: pinned left, right and bottom at a fixed height; its bottom pin
        // is what moves, from below the stage's edge to just inside it.
        Box::new(section("A panel that slides up", panel())),
        // A card: its left and top pins change, so it travels diagonally.
        Box::new(section("A card that travels", travelling_card())),
    ])
}

fn panel() -> Component {
    component(|cx| {
        let open = cx.use_state(|| false);
        let is_open = open.get();
        let scheme = &cx.theme().scheme;

        let sheet = Container::new()
            .radius(12.0)
            .padding(12.0)
            .color(scheme.primary_container)
            .child(Text::new("Details panel").color(scheme.on_primary_container));

        let stage = Stack::new()
            .width(300.0)
            .height(STAGE_HEIGHT)
            // `Expand`: the layers that are not positioned fill the stack — the page behind.
            .fit(StackFit::Expand)
            .layer(
                Container::new()
                    .radius(12.0)
                    .color(scheme.surface_container_high)
                    .padding(12.0)
                    .child(Text::new("Page content")),
            )
            .layer(
                // Closed, the panel waits just below the stage: a stack clips its layers
                // to its own edges, so it is out of sight there. Animating a height to zero instead would squeeze its padding and text
                // into nothing, and they would spill out of the shrinking box.
                AnimatedPositioned::new(0.3, Curve::ease_out(), sheet)
                    .left(8.0)
                    .right(8.0)
                    .bottom(if is_open { 8.0 } else { -PANEL_HEIGHT })
                    .height(PANEL_HEIGHT),
            );

        let toggle = open.clone();
        Box::new(
            Column::new()
                .spacing(12.0)
                .cross_axis_alignment(Align::Start)
                .child(
                    Button::new(if is_open { "Close panel" } else { "Open panel" })
                        .variant(Variant::Tonal)
                        .on_press(move || toggle.update(|open| *open = !*open)),
                )
                .child(stage),
        )
    })
}

fn travelling_card() -> Component {
    component(|cx| {
        let moved = cx.use_state(|| false);
        let far = moved.get();
        let scheme = &cx.theme().scheme;

        let card = Container::new()
            .radius(10.0)
            .color(scheme.tertiary)
            .alignment(Alignment::CENTER)
            .child(Text::new("Card").color(scheme.on_tertiary));

        let stage = Stack::new()
            .width(300.0)
            .height(160.0)
            .fit(StackFit::Expand)
            .layer(
                Container::new()
                    .radius(12.0)
                    .color(scheme.surface_container_high),
            )
            .layer(
                AnimatedPositioned::new(0.6, Curve::ease_in_out(), card)
                    .left(if far { 200.0 } else { 12.0 })
                    .top(if far { 96.0 } else { 12.0 })
                    .width(88.0)
                    .height(52.0),
            );

        let toggle = moved.clone();
        Box::new(
            Column::new()
                .spacing(12.0)
                .cross_axis_alignment(Align::Start)
                .child(
                    Row::new().child(
                        Button::new("Move")
                            .variant(Variant::Outlined)
                            .on_press(move || toggle.update(|far| *far = !*far)),
                    ),
                )
                .child(stage),
        )
    })
}
