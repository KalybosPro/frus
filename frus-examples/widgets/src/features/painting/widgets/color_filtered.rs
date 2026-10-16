//! `ColorFiltered` — changes the colours of everything a child paints: greyscale,
//! sepia, inverted, tinted.
//!
//! A filter is a `ColorFilter` value: a 5 × 4 colour matrix, with ready-made ones for
//! the usual effects, or a colour blended in with a `BlendMode`.

use frus::{
    component, Align, Alignment, BlendMode, BoxDecoration, Color, ColorFilter, ColorFiltered,
    Column, Container, LinearGradient, Switch, Text, Widget, Wrap,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        Box::new(section(
            "Matrices",
            Wrap::new()
                .gap(12.0)
                .run_gap(12.0)
                .child(labelled("original", picture()))
                .child(labelled(
                    "grayscale",
                    ColorFiltered::grayscale().child(picture()),
                ))
                .child(labelled(
                    "sepia",
                    ColorFiltered::new(ColorFilter::sepia()).child(picture()),
                ))
                .child(labelled(
                    "invert",
                    ColorFiltered::new(ColorFilter::invert()).child(picture()),
                ))
                .child(labelled(
                    "saturate 2×",
                    ColorFiltered::new(ColorFilter::saturate(2.0)).child(picture()),
                ))
                .child(labelled(
                    "brightness",
                    ColorFiltered::new(ColorFilter::brightness(0.25)).child(picture()),
                )),
        )),
        // `mode(colour, blend)` blends a colour into every pixel: a multiply tint here.
        Box::new(section(
            "Blend a colour in",
            Wrap::new()
                .gap(12.0)
                .run_gap(12.0)
                .child(labelled(
                    "multiply teal",
                    ColorFiltered::mode(Color::hex("#14B8A6"), BlendMode::Multiply)
                        .child(picture()),
                ))
                .child(labelled(
                    "screen amber",
                    ColorFiltered::mode(Color::hex("#F59E0B"), BlendMode::Screen).child(picture()),
                )),
        )),
        // `enabled(false)` removes the filter entirely (no layer at all) — the way to
        // switch an effect off, here a "greyed out" state.
        Box::new(section("Switched by state", toggled())),
    ])
}

fn toggled() -> impl Widget {
    component(|cx| {
        let grey = cx.use_state(|| true);
        let on = grey.get();
        Box::new(
            Column::new()
                .spacing(8.0)
                .cross_axis_alignment(Align::Start)
                .child(ColorFiltered::grayscale().enabled(on).child(picture()))
                .child(Switch::new(on).on_toggle(move |value| grey.set(value))),
        )
    })
}

/// A colourful 100 × 70 stand-in for a photo.
fn picture() -> Container {
    Container::new()
        .width(100.0)
        .height(70.0)
        .decoration(
            BoxDecoration::default().radius(8.0).gradient(
                LinearGradient::new(vec![
                    Color::hex("#EF4444"),
                    Color::hex("#EAB308"),
                    Color::hex("#22C55E"),
                    Color::hex("#3B82F6"),
                ])
                .between(Alignment::TOP_LEFT, Alignment::BOTTOM_RIGHT),
            ),
        )
        .alignment(Alignment::CENTER)
        .child(Text::new("photo").color(Color::hex("#FFFFFF")))
}

fn labelled(label: &str, child: impl Widget + 'static) -> Column {
    Column::new()
        .spacing(4.0)
        .child(child)
        .child(Text::new(label).size(12.0))
}
