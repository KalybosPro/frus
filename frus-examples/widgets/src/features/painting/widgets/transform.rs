//! `Transform` and `FractionalTranslation` — move, scale or rotate a child's painting
//! without changing the layout around it.
//!
//! The neighbours still see the untransformed box; use `RotatedBox` when the layout
//! should turn too. Taps follow the transformed child.

use frus::{
    component, Alignment, Clip, Color, Column, Container, Curve, FractionalTranslation, Slider,
    Stack, StackFit, Text, Transform, Widget, Wrap,
};

use crate::common::shared::{section, sections};

pub fn example() -> Box<dyn Widget> {
    sections(vec![
        Box::new(section(
            "Translate, scale, rotate",
            Wrap::new()
                .gap(40.0)
                .run_gap(24.0)
                .child(slot(Transform::translate(16.0, 8.0).child(card("moved"))))
                .child(slot(Transform::scale(0.7).child(card("scaled"))))
                .child(slot(Transform::rotate(0.3).child(card("rotated"))))
                // Combined, and turned about a corner instead of the centre.
                .child(slot(
                    Transform::rotate_from(-0.25, Alignment::BOTTOM_LEFT)
                        .and_scale(0.9)
                        .child(card("both")),
                )),
        )),
        // Live: the slider sets the angle; `animated` eases to each new value — the way a
        // pressed button shrinks a little, or a chevron turns when a panel opens.
        Box::new(section("Driven by state, animated", spinner())),
        // `FractionalTranslation` moves by a fraction of the child's own size: (0.5, 0)
        // shifts it right by half its width, whatever that width is.
        Box::new(section(
            "FractionalTranslation",
            Container::new()
                .padding_each(8.0, 120.0, 8.0, 8.0)
                .color(Color::hex("#EEF2FF"))
                .child(FractionalTranslation::new(0.5, 0.0).child(card("half my width →"))),
        )),
    ])
}

fn spinner() -> impl Widget {
    component(|cx| {
        let degrees = cx.use_state(|| 0.0_f32);
        let angle = degrees.get();
        Box::new(
            Column::new()
                .spacing(16.0)
                .child(slot(
                    Transform::rotate(angle.to_radians())
                        .animated(0.2, Curve::ease_out())
                        .child(card(&format!("{angle:.0}°"))),
                ))
                .child(
                    Slider::new(angle)
                        .range(0.0, 360.0)
                        .width(260.0)
                        .on_change(move |value| degrees.set(value)),
                ),
        )
    })
}

/// A fixed slot with a faint outline: the box the layout sees. The outline is a layer of
/// its own, under the child, so its border does not take room from the card; and the
/// stack does not clip, so the transformed card can paint past the slot.
fn slot(child: impl Widget + 'static) -> Stack {
    Stack::new()
        .width(100.0)
        .height(64.0)
        .fit(StackFit::Expand)
        .clip_behavior(Clip::None)
        .layer(Container::new().border(1.0, Color::hex("#CBD5E1")))
        .layer(child)
}

fn card(label: &str) -> Container {
    Container::new()
        .width(100.0)
        .height(64.0)
        .radius(8.0)
        .color(Color::hex("#6366F1").fade(0.85))
        .alignment(Alignment::CENTER)
        .child(Text::new(label).color(Color::hex("#FFFFFF")))
}
