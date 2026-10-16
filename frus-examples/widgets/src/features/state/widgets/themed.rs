//! `Themed` and `ThemeBuilder` — a different theme for one part of the screen.
//!
//! `Themed::new` replaces the theme for a subtree (a dark promo card on a light page).
//! `Themed::tweak` changes **part** of the inherited one (this section's buttons are
//! teal), keeping everything else. `ThemeBuilder` builds a subtree from whatever theme
//! is in force where it sits.

use frus::{
    component, Align, Button, Card, Color, Colors, Column, Container, Text, Theme, ThemeBuilder,
    Themed, Variant, Widget, Wrap,
};

use crate::common::shared::{section, sections};

/// A few widgets that only read the theme: shown under each theme below.
fn sample() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let scheme = &cx.theme().scheme;
        Box::new(
            Container::new()
                .width(260.0)
                .padding(16.0)
                .radius(12.0)
                .color(scheme.surface_container)
                .child(
                    Column::new()
                        .spacing(10.0)
                        .cross_axis_alignment(Align::Start)
                        .child(Text::new("Weekly digest").color(scheme.on_surface))
                        .child(
                            Wrap::new()
                                .gap(8.0)
                                .run_gap(8.0)
                                .child(Button::new("Open").on_press(|| {}))
                                .child(
                                    Button::new("Later")
                                        .variant(Variant::Outlined)
                                        .on_press(|| {}),
                                ),
                        ),
                ),
        )
    }))
}

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        Box::new(section("The theme around it", sample())),
        // A whole theme, made from one seed colour, dark whatever the app is.
        Box::new(section(
            "Themed::new(Theme::from_seed(purple, dark))",
            Themed::new(Theme::from_seed(Color::hex("#7C3AED"), true), sample()),
        )),
        // Only the buttons change; the surface and text stay the inherited ones.
        Box::new(section(
            "Themed::tweak: teal buttons only",
            Themed::tweak(
                |theme| {
                    theme.widgets.button.color = Some(Color::hex("#0F766E"));
                    theme.widgets.button.label_color = Some(Colors::WHITE);
                },
                sample(),
            ),
        )),
        // Built late, from the theme in force *here* — inside the dark `Themed`, the
        // builder sees the dark theme.
        Box::new(section(
            "ThemeBuilder: build from the ambient theme",
            Themed::new(
                Theme::from_seed(Color::hex("#0EA5E9"), true),
                ThemeBuilder::new(|theme| {
                    Card::new().padding(16.0).child(
                        Text::new(format!("Built under a {:?} theme", theme.brightness()))
                            .color(theme.scheme.on_surface),
                    )
                }),
            ),
        )),
    ])
}
