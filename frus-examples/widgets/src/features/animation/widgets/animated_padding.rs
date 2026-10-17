//! `AnimatedPadding` — insets its child by an amount that moves to each new value, so a
//! selected row can step in rather than jump.

use frus::{
    component, Align, AnimatedPadding, Column, Component, Container, Curve, Icon, Icons, Row, Text,
    Widget,
};

use crate::common::shared::{section, sections};

const FOLDERS: [&str; 4] = ["Inbox", "Starred", "Sent", "Drafts"];

pub fn example() -> Box<dyn Widget> {
    sections(vec![
        // Tap a row: the chosen one is indented further, and the change is animated.
        Box::new(section("Tap a folder", folders())),
    ])
}

fn folders() -> Component {
    component(|cx| {
        let selected = cx.use_state(|| 0_usize);
        let current = selected.get();
        let scheme = &cx.theme().scheme;

        let rows = FOLDERS.iter().enumerate().fold(
            Column::new()
                .spacing(4.0)
                .cross_axis_alignment(Align::Start),
            |column, (index, name)| {
                let chosen = index == current;
                let pick = selected.clone();
                column.child(
                    Container::new()
                        .width(260.0)
                        .radius(8.0)
                        .color(if chosen {
                            scheme.secondary_container
                        } else {
                            scheme.surface_container
                        })
                        .on_click(move || pick.set(index))
                        // Only the padding animates; the row's colour changes at once.
                        .child(AnimatedPadding::new(
                            if chosen { 28.0 } else { 12.0 },
                            0.25,
                            Curve::ease_out(),
                            Row::new()
                                .spacing(12.0)
                                .cross_axis_alignment(Align::Center)
                                .child(Icon::new(Icons::INBOX).size(20.0))
                                .child(Text::new(*name)),
                        )),
                )
            },
        );
        Box::new(rows)
    })
}
