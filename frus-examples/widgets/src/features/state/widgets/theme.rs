//! `Theme` — reading the colours and type every widget is dressed in (`cx.theme()`), and
//! switching the whole application between light and dark (`host::app()`).
//!
//! Read colours from the theme rather than writing them down: then dark mode, a brand
//! colour or a high-contrast theme reaches your widgets too.

use frus::{
    component, host, Align, Callback, Color, Column, Container, Row, SegmentedButton, Text, Theme,
    ThemeMode, Widget, Wrap,
};

use crate::common::shared::{section, sections};

const MODES: [(ThemeMode, &str); 3] = [
    (ThemeMode::Light, "Light"),
    (ThemeMode::Dark, "Dark"),
    (ThemeMode::System, "System"),
];

pub(crate) fn example() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let theme = cx.theme();
        let scheme = &theme.scheme;

        // The application's settings live in `host::app()`: the theme, the dark theme and
        // which of the two is on show. Changing one rebuilds the whole window.
        let app = host::app();
        let current = MODES
            .iter()
            .position(|(mode, _)| *mode == app.theme_mode())
            .unwrap_or(2);
        let modes = MODES.iter().fold(
            SegmentedButton::new(current, |index| {
                Callback::new(move || {
                    let app = host::app();
                    // An application with a single theme has nothing to switch to: give
                    // it a dark one the first time it is asked for.
                    if app.dark_theme().is_none() {
                        app.set_dark_theme(Some(Theme::dark()));
                    }
                    app.set_theme_mode(MODES[index].0);
                })
            }),
            |picker, (_, name)| picker.segment(*name),
        );

        let swatch = |name: &str, background: Color, foreground: Color| {
            Container::new()
                .width(132.0)
                .padding(10.0)
                .radius(8.0)
                .color(background)
                .border(1.0, scheme.outline_variant)
                .child(Text::new(name).size(12.0).color(foreground))
        };
        let colours = Wrap::new()
            .gap(8.0)
            .run_gap(8.0)
            .child(swatch("primary", scheme.primary, scheme.on_primary))
            .child(swatch(
                "primary_container",
                scheme.primary_container,
                scheme.on_primary_container,
            ))
            .child(swatch("secondary", scheme.secondary, scheme.on_secondary))
            .child(swatch("tertiary", scheme.tertiary, scheme.on_tertiary))
            .child(swatch("surface", scheme.surface, scheme.on_surface))
            .child(swatch(
                "surface_container_high",
                scheme.surface_container_high,
                scheme.on_surface,
            ))
            .child(swatch("error", scheme.error, scheme.on_error))
            .child(swatch(
                "inverse_surface",
                scheme.inverse_surface,
                scheme.on_inverse_surface,
            ));

        // The type scale: a named style for each role, rather than sizes picked by hand.
        let text = &theme.text;
        let type_scale = Column::new()
            .spacing(6.0)
            .cross_axis_alignment(Align::Start)
            .child(Text::styled("headline_small", text.headline_small))
            .child(Text::styled("title_medium", text.title_medium))
            .child(Text::styled("body_medium", text.body_medium))
            .child(Text::styled("label_small", text.label_small));

        sections(vec![
            Box::new(section(
                "Theme mode (the whole gallery follows)",
                Column::new()
                    .spacing(8.0)
                    .cross_axis_alignment(Align::Start)
                    .child(modes)
                    .child(
                        Text::new(format!("Building under a {:?} theme", theme.brightness()))
                            .size(13.0),
                    ),
            )),
            Box::new(section("Colour roles: cx.theme().scheme", colours)),
            Box::new(section(
                "Type roles: cx.theme().text",
                Row::new().child(type_scale),
            )),
        ])
    }))
}
