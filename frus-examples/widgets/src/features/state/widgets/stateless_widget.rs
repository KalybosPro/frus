//! `StatelessWidget`, `component` and `Builder` — a piece of interface pulled out into a
//! widget of its own, built from its configuration and nothing else.
//!
//! A struct with `build`, made again (cheaply) on every rebuild of its parent; or, for a
//! one-off, a closure: `frus::component(|cx| ..)` — which can also keep state with hooks.

use frus::{
    component, Align, BuildContext, Builder, Column, Container, Row, StatelessWidget, Text, Widget,
};

use crate::common::shared::{section, sections};

/// A reusable badge: what it shows is its configuration.
struct StatusBadge {
    label: &'static str,
    ok: bool,
}

impl StatelessWidget for StatusBadge {
    fn build(&self, cx: &BuildContext) -> Box<dyn Widget> {
        // The context gives the theme it is built under.
        let scheme = &cx.theme().scheme;
        let (background, foreground) = if self.ok {
            (scheme.primary_container, scheme.on_primary_container)
        } else {
            (scheme.error_container, scheme.on_error_container)
        };
        Box::new(
            Container::new()
                .padding(8.0)
                .radius(16.0)
                .color(background)
                .child(Text::new(self.label).size(13.0).color(foreground)),
        )
    }
}

pub fn example() -> Box<dyn Widget> {
    sections(vec![
        // `.into_widget()` turns the struct into a widget for any child slot.
        Box::new(section(
            "A StatelessWidget struct, configured three times",
            Row::new()
                .spacing(8.0)
                .child(
                    StatusBadge {
                        label: "Build passing",
                        ok: true,
                    }
                    .into_widget(),
                )
                .child(
                    StatusBadge {
                        label: "Tests passing",
                        ok: true,
                    }
                    .into_widget(),
                )
                .child(
                    StatusBadge {
                        label: "Lint failing",
                        ok: false,
                    }
                    .into_widget(),
                ),
        )),
        // A closure is a component too. `Builder::new` is the same thing under another
        // name: a place in the tree with a context of its own.
        Box::new(section(
            "A closure: component(|cx| ..) / Builder::new(|cx| ..)",
            Column::new()
                .spacing(8.0)
                .cross_axis_alignment(Align::Start)
                .child(component(|cx| {
                    Box::new(Text::new("Built by a closure").color(cx.theme().primary))
                }))
                .child(Builder::new(|cx| {
                    Box::new(
                        Text::new(format!("Theme brightness: {:?}", cx.theme().brightness()))
                            .size(13.0),
                    )
                })),
        )),
    ])
}
