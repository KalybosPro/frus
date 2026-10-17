//! `IgnorePointer` and `AbsorbPointer` — switch off the pointer for a subtree without
//! touching the widgets in it.
//!
//! Both leave the subtree drawn exactly as it is. The difference is what happens to a tap:
//! `IgnorePointer` lets it **through** to whatever is behind; `AbsorbPointer` **stops** it,
//! so nothing behind gets it either.

use frus::{
    component, AbsorbPointer, Align, Alignment, Button, Column, Container, IgnorePointer, Row,
    Stack, StackFit, Switch, Text, Widget,
};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let blocked = cx.use_state(|| true);
        let front = cx.use_state(|| 0_u32);
        let behind = cx.use_state(|| 0_u32);
        let on = blocked.get();
        let scheme = &cx.theme().scheme;

        // A card behind, counting the taps that reach it, with a button in front of it.
        let scene = |front_button: Box<dyn Widget>| {
            let hit = behind.clone();
            Stack::new()
                .width(260.0)
                .height(110.0)
                .fit(StackFit::Expand)
                .layer(
                    Container::new()
                        .radius(12.0)
                        .color(scheme.surface_container_high)
                        .alignment(Alignment::BOTTOM_LEFT)
                        .padding(10.0)
                        .on_click(move || hit.update(|n| *n += 1))
                        .child(Text::new("Behind: tap anywhere").size(12.0)),
                )
                .layer(
                    Container::new()
                        .alignment(Alignment::TOP_CENTER)
                        .padding(12.0)
                        .child(front_button),
                )
        };
        let button = |label: &str| {
            let tap = front.clone();
            Button::new(label).on_press(move || tap.update(|n| *n += 1))
        };

        // While on, the button cannot be pressed — and a tap on it reaches the card behind.
        let ignoring = scene(Box::new(IgnorePointer::new(button("Ignored")).ignoring(on)));
        // While on, the button cannot be pressed — and the tap stops there.
        let absorbing = scene(Box::new(
            AbsorbPointer::new(button("Absorbed")).absorbing(on),
        ));

        let flip = blocked.clone();
        sections(vec![
            Box::new(section(
                "Blocking",
                Row::new()
                    .spacing(12.0)
                    .cross_axis_alignment(Align::Center)
                    .child(Switch::new(on).on_toggle(move |value| flip.set(value)))
                    .child(Text::new(if on { "On" } else { "Off" })),
            )),
            Box::new(section(
                "Counts",
                Column::new()
                    .spacing(4.0)
                    .cross_axis_alignment(Align::Start)
                    .child(Text::new(format!("Button pressed: {}", front.get())))
                    .child(Text::new(format!("Card behind tapped: {}", behind.get()))),
            )),
            Box::new(section("IgnorePointer: the tap goes through", ignoring)),
            Box::new(section("AbsorbPointer: the tap stops here", absorbing)),
        ])
    }))
}
