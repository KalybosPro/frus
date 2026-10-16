//! `AnimatedFractionallySizedBox` — a box sized as a fraction of the room it is given,
//! moving to each new fraction: a progress bar, a panel that widens.

use frus::{
    component, Align, Alignment, AnimatedFractionallySizedBox, Button, Column, Component,
    Container, Curve, Text, Variant, Widget, Wrap,
};

use crate::common::shared::{section, sections};

const STEPS: [f32; 4] = [0.1, 0.4, 0.75, 1.0];

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        // The track is 280 px wide; the fill is a fraction of it, whatever the track's size.
        Box::new(section("Upload progress", progress())),
    ])
}

fn progress() -> Component {
    component(|cx| {
        let step = cx.use_state(|| 0_usize);
        let fraction = STEPS[step.get()];
        let scheme = &cx.theme().scheme;

        let buttons = STEPS.iter().enumerate().fold(
            Wrap::new().gap(8.0).run_gap(8.0),
            |row, (index, value)| {
                let pick = step.clone();
                row.child(
                    Button::new(format!("{:.0} %", value * 100.0))
                        .variant(if index == step.get() {
                            Variant::Filled
                        } else {
                            Variant::Outlined
                        })
                        .on_press(move || pick.set(index)),
                )
            },
        );

        Box::new(
            Column::new()
                .spacing(16.0)
                .cross_axis_alignment(Align::Start)
                .child(buttons)
                .child(
                    Container::new()
                        .width(280.0)
                        .height(16.0)
                        .radius(8.0)
                        .color(scheme.surface_container_highest)
                        .alignment(Alignment::CENTER_LEFT)
                        .child(
                            AnimatedFractionallySizedBox::new(
                                0.5,
                                Curve::ease_out(),
                                Container::new().radius(8.0).color(scheme.primary),
                            )
                            .width_factor(fraction)
                            .height_factor(1.0),
                        ),
                )
                .child(Text::new(format!("{:.0} % uploaded", fraction * 100.0)).size(13.0)),
        )
    })
}
