//! `Stack` and `Positioned` — children layered in the same box, the last on top: a
//! badge on an avatar, a caption over a picture, a button floating over content.

use frus::{Alignment, Clip, Color, Container, Positioned, Stack, StackFit, Text, Widget, Wrap};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        // Layers that are not positioned sit at the stack's `alignment`; the stack is as
        // big as the biggest of them.
        Box::new(section(
            "Layers, aligned",
            Wrap::new()
                .gap(16.0)
                .run_gap(16.0)
                .child(squares(Alignment::TOP_LEFT))
                .child(squares(Alignment::CENTER))
                .child(squares(Alignment::BOTTOM_RIGHT)),
        )),
        // `Positioned` pins a layer against the stack's edges: a badge in the top-right
        // corner of an avatar. A layer that goes past the edge is clipped unless the
        // stack's `clip_behavior` says `Clip::None`.
        Box::new(section(
            "Positioned badge",
            Stack::new()
                .clip_behavior(Clip::None)
                .layer(
                    Container::new()
                        .width(64.0)
                        .height(64.0)
                        .radius(32.0)
                        .color(Color::hex("#6366F1"))
                        .alignment(Alignment::CENTER)
                        .child(Text::new("AB").color(Color::hex("#FFFFFF"))),
                )
                .layer(
                    Positioned::new(
                        Container::new()
                            .padding_each(2.0, 6.0, 2.0, 6.0)
                            .radius(10.0)
                            .color(Color::hex("#EF4444"))
                            .child(Text::new("3").size(12.0).color(Color::hex("#FFFFFF"))),
                    )
                    .top(-4.0)
                    .right(-4.0),
                ),
        )),
        // Pinning two opposite edges sizes the layer: `left` + `right` makes a caption bar
        // as wide as the picture, `bottom` puts it at the foot. `Positioned::fill` covers
        // the whole stack — a tint over a picture.
        Box::new(section(
            "Caption over a picture",
            Stack::new()
                .width(280.0)
                .height(160.0)
                .fit(StackFit::Expand)
                .layer(Container::new().radius(8.0).color(Color::hex("#0EA5E9")))
                .layer(Positioned::fill(
                    Container::new()
                        .radius(8.0)
                        .color(Color::rgba(0.0, 0.0, 0.0, 0.1)),
                ))
                .layer(
                    Positioned::new(
                        Container::new()
                            .padding(10.0)
                            .color(Color::rgba(0.0, 0.0, 0.0, 0.55))
                            .child(Text::new("Lake at dawn").color(Color::hex("#FFFFFF"))),
                    )
                    .left(0.0)
                    .right(0.0)
                    .bottom(0.0),
                )
                .layer(
                    Positioned::new(
                        Container::new()
                            .padding_each(4.0, 8.0, 4.0, 8.0)
                            .radius(4.0)
                            .color(Color::hex("#FFFFFF"))
                            .child(Text::new("NEW").size(11.0).color(Color::hex("#0369A1"))),
                    )
                    .top(8.0)
                    .start(8.0),
                ),
        )),
    ])
}

/// Three squares of decreasing size, layered and aligned the same way.
fn squares(alignment: Alignment) -> Stack {
    Stack::new()
        .alignment(alignment)
        .layer(
            Container::new()
                .width(96.0)
                .height(96.0)
                .color(Color::hex("#C7D2FE")),
        )
        .layer(
            Container::new()
                .width(64.0)
                .height(64.0)
                .color(Color::hex("#818CF8")),
        )
        .layer(
            Container::new()
                .width(32.0)
                .height(32.0)
                .color(Color::hex("#4F46E5")),
        )
}
