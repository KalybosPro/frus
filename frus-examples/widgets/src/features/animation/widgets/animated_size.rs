//! `AnimatedSize` — a box that follows its child's size over time: when the content grows
//! or shrinks, the box moves to the new size instead of jumping, and clips the child on
//! the way.

use frus::{
    component, Align, AnimatedSize, Button, Column, Component, Container, Curve, Text, Variant,
    Widget,
};

use crate::common::shared::{section, sections};

const SUMMARY: &str = "frus draws every pixel itself, on the GPU.";
const MORE: &str = "That is why a widget looks the same on the desktop, on Android and in a \
                    browser: no native control is borrowed, so nothing changes from one \
                    platform to the next except what the platform should change — scrolling \
                    physics, page transitions, the text selection handles.";

pub fn example() -> Box<dyn Widget> {
    sections(vec![
        // "Read more": the paragraph's box opens and closes smoothly; what is below it
        // follows the box.
        Box::new(section("Read more", read_more())),
    ])
}

fn read_more() -> Component {
    component(|cx| {
        let expanded = cx.use_state(|| false);
        let open = expanded.get();
        let scheme = &cx.theme().scheme;

        // The child changes; `AnimatedSize` notices on the next layout and animates its
        // own box from the old size to the new one.
        let mut body = Column::new()
            .spacing(8.0)
            .cross_axis_alignment(Align::Start)
            .child(Text::new(SUMMARY));
        if open {
            body = body.child(Text::new(MORE).color(scheme.on_surface_variant));
        }

        let toggle = expanded.clone();
        Box::new(
            Column::new()
                .spacing(12.0)
                .cross_axis_alignment(Align::Start)
                .child(
                    Container::new()
                        .width(300.0)
                        .padding(12.0)
                        .radius(12.0)
                        .color(scheme.surface_container)
                        .child(AnimatedSize::new(0.3, body).curve(Curve::ease_in_out())),
                )
                .child(
                    Button::new(if open { "Read less" } else { "Read more" })
                        .variant(Variant::Text)
                        .on_press(move || toggle.update(|open| *open = !*open)),
                ),
        )
    })
}
