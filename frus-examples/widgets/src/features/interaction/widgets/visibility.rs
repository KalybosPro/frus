//! `Visibility` and `Offstage` — hide a child, and choose how much of it survives being
//! hidden: its box, its input, or nothing at all.

use frus::{component, Align, Column, Container, Offstage, Row, Switch, Text, Visibility, Widget};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let shown = cx.use_state(|| true);
        let on = shown.get();
        let scheme = &cx.theme().scheme;

        // The same three boxes in each row; only the middle one is hidden.
        let block = |label: &str, highlighted: bool| {
            Container::new()
                .padding(10.0)
                .radius(8.0)
                .color(if highlighted {
                    scheme.primary_container
                } else {
                    scheme.surface_container_high
                })
                .child(Text::new(label).size(13.0))
        };
        let row = |middle: Box<dyn Widget>| {
            Row::new()
                .spacing(8.0)
                .cross_axis_alignment(Align::Center)
                .child(block("Left", false))
                .child_boxed(middle)
                .child(block("Right", false))
        };

        let flip = shown.clone();
        sections(vec![
            Box::new(section(
                "Middle box visible",
                Row::new()
                    .spacing(12.0)
                    .cross_axis_alignment(Align::Center)
                    .child(Switch::new(on).on_toggle(move |value| flip.set(value)))
                    .child(Text::new(if on { "Shown" } else { "Hidden" })),
            )),
            Box::new(section(
                "Visibility: hidden collapses the space",
                Column::new().child(row(Box::new(
                    Visibility::new(block("Middle", true)).visible(on),
                ))),
            )),
            // A spinner that comes and goes beside a label: keep its box so nothing moves.
            Box::new(section(
                "Visibility::maintain_size: hidden keeps the space",
                row(Box::new(
                    Visibility::new(block("Middle", true))
                        .visible(on)
                        .maintain_size(),
                )),
            )),
            Box::new(section(
                "Visibility::replacement: something else in its place",
                row(Box::new(
                    Visibility::new(block("Middle", true))
                        .visible(on)
                        .replacement(Text::new("…").size(13.0)),
                )),
            )),
            // Out of the layout entirely; its state is not kept while it is gone.
            Box::new(section(
                "Offstage: gone from layout, paint and input",
                row(Box::new(Offstage::new(block("Middle", true)).offstage(!on))),
            )),
        ])
    }))
}
