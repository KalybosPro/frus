//! `AnimatedCrossFade` — shows one of two children and cross-fades to the other when told
//! to, the box moving between their two sizes as they fade.
//!
//! Reach for it when the two states are known in advance (collapsed / expanded, a
//! summary / its details); `AnimatedSwitcher` is for a value that can be anything.

use frus::{
    component, Align, AnimatedCrossFade, Button, Column, Container, Curve, Icon, Icons, Row, Text,
    Variant, Widget,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let expanded = cx.use_state(|| false);
        let open = expanded.get();
        let scheme = &cx.theme().scheme;

        // The first child: one line.
        let summary = Row::new()
            .spacing(8.0)
            .cross_axis_alignment(Align::Center)
            .child(Icon::new(Icons::INBOX).size(20.0))
            .child(Text::new("3 new messages"));

        // The second: taller, with more in it. The box grows from one height to the other
        // while the two fade.
        let details = Column::new()
            .spacing(6.0)
            .cross_axis_alignment(Align::Start)
            .child(Text::new("Ada — Lunch on Friday?"))
            .child(Text::new("Grace — The build is green"))
            .child(Text::new("Alan — Notes from the meeting"));

        let card = Container::new()
            .width(300.0)
            .padding(16.0)
            .radius(12.0)
            .color(scheme.surface_container)
            .child(
                AnimatedCrossFade::new(summary, details, !open, 0.3)
                    // Each child fades on its own curve, and the size on a third.
                    .first_curve(Curve::ease_out())
                    .second_curve(Curve::ease_in())
                    .size_curve(Curve::ease_in_out()),
            );

        let toggle = expanded.clone();
        sections(vec![Box::new(section(
            "Summary and details",
            Column::new()
                .spacing(12.0)
                .cross_axis_alignment(Align::Start)
                .child(card)
                .child(
                    Button::new(if open { "Collapse" } else { "Expand" })
                        .variant(Variant::Tonal)
                        .on_press(move || toggle.update(|open| *open = !*open)),
                ),
        ))])
    }))
}
