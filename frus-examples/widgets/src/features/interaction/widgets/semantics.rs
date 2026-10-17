//! `Semantics`, `ExcludeSemantics` — what a screen reader is told about a widget, said
//! from outside it.
//!
//! Built-in widgets describe themselves (a button says it is a button, with its label).
//! These wrappers are for what only you know: an icon-only control's name, a row that is
//! really one thing, a heading made of a widget, a decoration that should stay silent.
//! Nothing changes on screen — turn on a screen reader (Narrator, TalkBack) to hear it.

use frus::{
    component, Align, Alignment, Column, Container, ExcludeSemantics, Icon, Icons, Role, Row,
    Semantics, SemanticsProperties, Text, Widget,
};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let liked = cx.use_state(|| false);
        let on = liked.get();
        let scheme = &cx.theme().scheme;

        // A home-made toggle drawn with a bare icon: without help a reader finds nothing
        // to announce. Give it a role, a name, its state, and say it can be activated.
        let flip = liked.clone();
        let heart = Semantics::new(
            SemanticsProperties::new(Role::Switch),
            Container::new()
                .padding(10.0)
                .radius(24.0)
                .color(scheme.surface_container_high)
                .on_click(move || flip.update(|on| *on = !*on))
                .child(Icon::new(Icons::FAVORITE).color(if on {
                    scheme.error
                } else {
                    scheme.outline
                })),
        )
        .label("Favourite")
        .toggled(on)
        .clickable(true);

        // A heading made of several widgets: announced once, as a landmark.
        let heading = Semantics::heading(
            Row::new()
                .spacing(8.0)
                .cross_axis_alignment(Align::Center)
                .child(Icon::new(Icons::INBOX).color(scheme.primary))
                .child(Text::new("Inbox").size(20.0)),
        );

        // A row of separate texts that is really one statement: merged, it is read as
        // one item rather than three.
        let merged = Semantics::merge(
            Column::new()
                .spacing(2.0)
                .cross_axis_alignment(Align::Start)
                .child(Text::new("Ada Lovelace"))
                .child(Text::new("Lunch on Friday?").size(13.0))
                .child(
                    Text::new("10:42")
                        .size(12.0)
                        .color(scheme.on_surface_variant),
                ),
        );

        // A purely decorative picture: nothing to say, so say nothing.
        let decoration = ExcludeSemantics::new(
            Container::new()
                .width(120.0)
                .height(24.0)
                .radius(12.0)
                .color(scheme.tertiary_container)
                .alignment(Alignment::CENTER)
                .child(Text::new("~ ~ ~").color(scheme.on_tertiary_container)),
        );

        sections(vec![
            Box::new(section(
                "An icon-only toggle, named",
                Row::new()
                    .spacing(12.0)
                    .cross_axis_alignment(Align::Center)
                    .child(heart)
                    .child(
                        Text::new(format!(
                            "Read as: \"Favourite, switch, {}\"",
                            if on { "on" } else { "off" }
                        ))
                        .size(13.0),
                    ),
            )),
            Box::new(section("A heading made of widgets", heading)),
            Box::new(section("Three texts read as one", merged)),
            Box::new(section("Decoration left out", decoration)),
        ])
    }))
}
