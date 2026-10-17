//! The text theme (`theme.text`) — the app's type scale: fifteen named styles from
//! `display_large` down to `label_small`, and `Text::heading` for titles.
//!
//! Use the scale instead of raw sizes, so every screen shares one hierarchy and a theme
//! change restyles the whole app.

use frus::{component, Align, Column, Text, TextStyle, Widget};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    sections(vec![
        // A page title and a section title, from the scale. `heading()` changes nothing
        // drawn: it tells screen readers this is a title, so their users can jump between
        // headings.
        Box::new(section("Headings", headings())),
        Box::new(section("The whole scale", scale())),
    ])
}

/// The theme is read from the build context, so a component is needed to reach it.
fn headings() -> impl Widget {
    component(|cx| {
        let text = &cx.theme().text;
        Box::new(
            Column::new()
                .spacing(8.0)
                .cross_axis_alignment(Align::Start)
                .child(Text::styled("Account settings", text.headline_small).heading())
                .child(Text::styled("Notifications", text.title_medium).heading())
                .child(Text::styled(
                    "Choose which messages reach you, and how.",
                    text.body_medium,
                )),
        )
    })
}

fn scale() -> impl Widget {
    component(|cx| {
        let text = &cx.theme().text;
        let styles: [(&str, TextStyle); 15] = [
            ("display_large", text.display_large),
            ("display_medium", text.display_medium),
            ("display_small", text.display_small),
            ("headline_large", text.headline_large),
            ("headline_medium", text.headline_medium),
            ("headline_small", text.headline_small),
            ("title_large", text.title_large),
            ("title_medium", text.title_medium),
            ("title_small", text.title_small),
            ("body_large", text.body_large),
            ("body_medium", text.body_medium),
            ("body_small", text.body_small),
            ("label_large", text.label_large),
            ("label_medium", text.label_medium),
            ("label_small", text.label_small),
        ];
        let column = Column::new()
            .spacing(6.0)
            .cross_axis_alignment(Align::Start);
        // The display sizes are wide: let them wrap rather than run off a phone screen.
        Box::new(styles.into_iter().fold(column, |column, (name, style)| {
            column.child(Text::styled(name, style).wrap())
        }))
    })
}
