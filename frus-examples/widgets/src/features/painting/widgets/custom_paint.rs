//! `CustomPaint` — a fixed-size canvas you draw yourself: charts, gauges, signatures,
//! anything no widget draws for you.
//!
//! The closure gets the `Scene` to draw into, its box on screen (`Rect`, in window
//! coordinates — offset everything by `bounds.x` / `bounds.y`) and the theme, so the
//! drawing follows light and dark mode like every widget does.

use frus::{
    component, Color, Column, CustomPaint, Path, Point, Rect, Slider, Stroke, StrokeCap, Widget,
    Wrap,
};

use crate::common::shared::{section, sections};

const SALES: [f32; 7] = [0.35, 0.6, 0.45, 0.8, 0.7, 0.95, 0.55];

pub fn example() -> Box<dyn Widget> {
    sections(vec![
        // Rectangles and a polyline: a small chart in the theme's colours.
        Box::new(section(
            "A bar chart with a trend line",
            CustomPaint::new(280.0, 140.0, |scene, bounds, theme| {
                // The axis along the bottom.
                scene.fill_rect(
                    Rect::new(bounds.x, bounds.y + bounds.height - 1.0, bounds.width, 1.0),
                    theme.border,
                );
                let slot = bounds.width / SALES.len() as f32;
                let mut trend = Path::new();
                for (i, value) in SALES.iter().enumerate() {
                    let height = (bounds.height - 12.0) * value;
                    let x = bounds.x + i as f32 * slot;
                    let top = bounds.y + bounds.height - 1.0 - height;
                    scene.fill_rect(
                        Rect::new(x + slot * 0.2, top, slot * 0.6, height),
                        theme.primary.fade(0.35),
                    );
                    let peak = Point::new(x + slot / 2.0, top);
                    trend = if i == 0 {
                        trend.move_to(peak)
                    } else {
                        trend.line_to(peak)
                    };
                }
                scene.paint_path(
                    &trend,
                    None,
                    Some(Stroke::new(theme.primary, 2.5).with_cap(StrokeCap::Round)),
                );
            }),
        )),
        Box::new(section(
            "Shapes: circles, arcs and curves",
            Wrap::new()
                .gap(16.0)
                .run_gap(16.0)
                .child(smiley())
                .child(CustomPaint::new(140.0, 100.0, |scene, b, theme| {
                    // A wave: one cubic Bézier curve.
                    let wave = Path::new()
                        .move_to(Point::new(b.x, b.y + b.height / 2.0))
                        .cubic_to(
                            Point::new(b.x + b.width * 0.35, b.y - 20.0),
                            Point::new(b.x + b.width * 0.65, b.y + b.height + 20.0),
                            Point::new(b.x + b.width, b.y + b.height / 2.0),
                        );
                    scene.stroke_path(&wave, theme.primary, 3.0);
                })),
        )),
        // A gauge redrawn from state: the canvas is rebuilt with the new value every time
        // the slider moves.
        Box::new(section("A gauge driven by state", gauge())),
    ])
}

fn smiley() -> CustomPaint {
    CustomPaint::new(100.0, 100.0, |scene, b, _theme| {
        let center = Point::new(b.x + b.width / 2.0, b.y + b.height / 2.0);
        scene.fill_path(&Path::circle(center, 46.0), Color::hex("#FACC15"));
        scene.fill_path(
            &Path::circle(Point::new(center.x - 16.0, center.y - 12.0), 6.0),
            Color::hex("#422006"),
        );
        scene.fill_path(
            &Path::circle(Point::new(center.x + 16.0, center.y - 12.0), 6.0),
            Color::hex("#422006"),
        );
        // An arc from 20° to 160°, below the centre (y grows downwards).
        let (start, end) = (20_f32.to_radians(), 160_f32.to_radians());
        let mouth = Path::new()
            .move_to(Point::new(
                center.x + 26.0 * start.cos(),
                center.y + 26.0 * start.sin(),
            ))
            .arc_to(center, 26.0, start, end);
        scene.paint_path(
            &mouth,
            None,
            Some(Stroke::new(Color::hex("#422006"), 4.0).with_cap(StrokeCap::Round)),
        );
    })
}

fn gauge() -> impl Widget {
    component(|cx| {
        let level = cx.use_state(|| 0.65_f32);
        let value = level.get();
        let canvas = CustomPaint::new(160.0, 100.0, move |scene, b, theme| {
            let center = Point::new(b.x + b.width / 2.0, b.y + b.height - 10.0);
            let radius = 64.0;
            let start = std::f32::consts::PI;
            let arc = |to: f32| {
                Path::new()
                    .move_to(Point::new(center.x - radius, center.y))
                    .arc_to(center, radius, start, start + std::f32::consts::PI * to)
            };
            let track = Stroke::new(theme.border, 12.0).with_cap(StrokeCap::Round);
            let fill = Stroke::new(theme.primary, 12.0).with_cap(StrokeCap::Round);
            scene.paint_path(&arc(1.0), None, Some(track));
            if value > 0.0 {
                scene.paint_path(&arc(value), None, Some(fill));
            }
        });
        Box::new(
            Column::new().spacing(8.0).child(canvas).child(
                Slider::new(value)
                    .width(220.0)
                    .on_change(move |v| level.set(v)),
            ),
        )
    })
}
