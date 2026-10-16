//! `AnimatedDefaultTextStyle` — the text style handed down to a subtree, moving to each new
//! style: size, weight and colour change smoothly for every text inside that does not set
//! them itself.

use frus::{
    component, Align, AnimatedDefaultTextStyle, Button, Column, Component, Curve, FontWeight, Text,
    TextStyle, Variant, Widget,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        // A heading that grows when its section is selected. The texts inside name no size,
        // weight or colour of their own — a text that states its size keeps it.
        Box::new(section("Selected heading", heading())),
    ])
}

fn heading() -> Component {
    component(|cx| {
        let selected = cx.use_state(|| false);
        let on = selected.get();
        let scheme = &cx.theme().scheme;

        let style = if on {
            TextStyle::NONE
                .size(28.0)
                .weight(FontWeight::Bold)
                .color(scheme.primary)
        } else {
            TextStyle::NONE
                .size(16.0)
                .weight(FontWeight::Regular)
                .color(scheme.on_surface_variant)
        };

        let toggle = selected.clone();
        Box::new(
            Column::new()
                .spacing(16.0)
                .cross_axis_alignment(Align::Start)
                .child(
                    Button::new(if on { "Deselect" } else { "Select" })
                        .variant(Variant::Tonal)
                        .on_press(move || toggle.update(|on| *on = !*on)),
                )
                .child(AnimatedDefaultTextStyle::new(
                    0.3,
                    Curve::ease_in_out(),
                    style,
                    Column::new()
                        .spacing(4.0)
                        .cross_axis_alignment(Align::Start)
                        .child(Text::new("Weekly report"))
                        .child(Text::new("Both lines follow the handed-down style")),
                )),
        )
    })
}
