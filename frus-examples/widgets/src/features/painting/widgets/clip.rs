//! `ClipRect`, `ClipRRect`, `ClipOval` and `ClipPath` — cut a child's painting to a
//! shape: rounded corners on a picture, a round avatar, a star.
//!
//! A clip changes what is painted, never the layout: the box keeps its size, only the
//! pixels outside the shape are dropped.

use frus::{
    Alignment, BorderRadius, BoxDecoration, ClipOval, ClipPath, ClipRRect, ClipRect, Color, Column,
    Container, LinearGradient, Padding, Path, Point, Text, Transform, Widget, Wrap,
};

use crate::common::shared::{section, sections};

pub(crate) fn example() -> Box<dyn Widget> {
    sections(vec![
        Box::new(section(
            "Shapes",
            Wrap::new()
                .gap(16.0)
                .run_gap(16.0)
                // Rounded corners — the same radius everywhere, or per corner.
                .child(labelled("ClipRRect", ClipRRect::new(16.0).child(picture())))
                .child(labelled(
                    "ClipRRect, top only",
                    ClipRRect::rounded(BorderRadius::top(32.0)).child(picture()),
                ))
                // The ellipse inscribed in the box: a circle when the box is square.
                .child(labelled("ClipOval", ClipOval::new().child(picture())))
                // Any path, in the box's own coordinates (origin at its top-left).
                .child(labelled(
                    "ClipPath",
                    ClipPath::new(star(100.0, 100.0)).child(picture()),
                )),
        )),
        // `ClipRect` cuts at the box's edges. A child that paints past them — here one
        // scaled up 1.3× by a transform — stays inside instead of covering its
        // neighbours.
        Box::new(section(
            "ClipRect",
            Padding::new(
                16.0,
                Wrap::new()
                    .gap(48.0)
                    .run_gap(16.0)
                    .child(labelled(
                        "unclipped",
                        Transform::scale(1.3).child(picture()),
                    ))
                    .child(labelled(
                        "clipped",
                        ClipRect::new().child(Transform::scale(1.3).child(picture())),
                    )),
            ),
        )),
    ])
}

/// A 100 × 100 stand-in for a photo: a gradient with a caption.
fn picture() -> Container {
    Container::new()
        .width(100.0)
        .height(100.0)
        .decoration(
            BoxDecoration::default().gradient(
                LinearGradient::new(vec![Color::hex("#22D3EE"), Color::hex("#6366F1")])
                    .between(Alignment::TOP_LEFT, Alignment::BOTTOM_RIGHT),
            ),
        )
        .alignment(Alignment::CENTER)
        .child(Text::new("photo").color(Color::hex("#FFFFFF")))
}

/// A five-pointed star filling a `width` × `height` box.
fn star(width: f32, height: f32) -> Path {
    let (cx, cy) = (width / 2.0, height / 2.0);
    let outer = width.min(height) / 2.0;
    let inner = outer * 0.45;
    (0..10)
        .fold(Path::new(), |path, i| {
            let radius = if i % 2 == 0 { outer } else { inner };
            // Start at the top and go round clockwise, a tenth of a turn per point.
            let angle = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
            let point = Point::new(cx + radius * angle.cos(), cy + radius * angle.sin());
            if i == 0 {
                path.move_to(point)
            } else {
                path.line_to(point)
            }
        })
        .close()
}

fn labelled(label: &str, child: impl Widget + 'static) -> Column {
    Column::new()
        .spacing(6.0)
        .child(child)
        .child(Text::new(label).size(12.0))
}
