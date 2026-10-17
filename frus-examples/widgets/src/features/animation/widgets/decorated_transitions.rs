//! `DecoratedBoxTransition` and `DefaultTextStyleTransition` — a whole decoration, or a
//! whole text style, a given fraction of the way from one to another.
//!
//! Every part moves by its own rule: colours blend, corners round, shadows grow, sizes and
//! weights step through. Here a slider owns the progress; a controller or a gesture would
//! do as well.

use frus::{
    component, Align, Alignment, BorderRadius, BoxDecoration, BoxShadow, Color, Container,
    DecoratedBoxTransition, DefaultTextStyleTransition, FontWeight, Row, Slider, Text, TextStyle,
    Widget,
};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    Box::new(component(|cx| {
        let progress = cx.use_state(|| 0.0_f32);
        let t = progress.get();
        let scheme = &cx.theme().scheme;

        // Two complete decorations: a flat square and a raised, rounded card.
        let flat =
            BoxDecoration::filled(scheme.surface_container_high).radius(BorderRadius::uniform(0.0));
        let raised = BoxDecoration::filled(scheme.primary_container)
            .radius(BorderRadius::uniform(28.0))
            .shadow(BoxShadow::new(
                0.0,
                8.0,
                24.0,
                Color::rgba(0.0, 0.0, 0.0, 0.25),
            ));

        // Two text styles; the text below names none of these properties itself, so it
        // wears whatever is handed down.
        let quiet = TextStyle::NONE
            .size(14.0)
            .weight(FontWeight::Regular)
            .color(scheme.on_surface_variant);
        let loud = TextStyle::NONE
            .size(24.0)
            .weight(FontWeight::Bold)
            .color(scheme.on_primary_container);

        let set = progress.clone();
        let slider = Row::new()
            .spacing(12.0)
            .cross_axis_alignment(Align::Center)
            .child(
                Slider::new(t)
                    .width(220.0)
                    .on_change(move |value| set.set(value)),
            )
            .child(Text::new(format!("{t:.2}")).size(13.0));

        sections(vec![
            Box::new(section("Progress", slider)),
            Box::new(section(
                "Decoration and text style, together",
                Container::new()
                    .padding(12.0)
                    .child(DecoratedBoxTransition::between(
                        flat,
                        raised,
                        t,
                        Container::new()
                            .width(240.0)
                            .height(120.0)
                            .alignment(Alignment::CENTER)
                            .child(DefaultTextStyleTransition::between(
                                quiet,
                                loud,
                                t,
                                Text::new("Featured"),
                            )),
                    )),
            )),
        ])
    }))
}
