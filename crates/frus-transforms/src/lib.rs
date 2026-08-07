//! `frus-transforms` — an **animated, interactive** showcase of the layout and
//! painting arsenal: the [`Transform`] palette (translation, non-uniform scale,
//! rotation, offset pivot, compositions), [`AspectRatio`] and
//! [`FractionallySizedBox`], driven by a [`Tween`] over time **and** by the user,
//! through a slider and buttons.
//!
//! Worth noting: a **clickable button placed inside a rotated `Transform`** still
//! responds — *visible* proof that hit-testing goes through transforms, by way of
//! the inverse matrix.
//!
//! The full Elm model: a small state, a pure `update`, a subscription keeping time
//! (about 60 fps, silent when paused), and a pure `view`.
//!
//! Run on desktop with `cargo run -p frus-transforms`.

use std::time::Duration;

use frus_shell::{Application, Command, Subscription};
use frus_widgets::{
    Align, Alignment, AspectRatio, BoxFit, Button, ClipOval, ClipPath, ClipRRect, Color, Container,
    Curve, FittedBox, Flex, FractionallySizedBox, InteractiveViewer, Justify, Path, Point,
    RotatedBox, Scroll, Slider, Text, Theme, Transform, Tween, Variant, Widget,
};

/// A five-pointed **star** path inscribed in a `size × size` box, in local
/// coordinates — for the `ClipPath` tile.
fn star_path(size: f32) -> Path {
    let c = size / 2.0;
    let (outer, inner) = (c * 0.98, c * 0.42);
    let mut p = Path::new();
    for i in 0..10 {
        let r = if i % 2 == 0 { outer } else { inner };
        let a = -std::f32::consts::FRAC_PI_2 + (i as f32) * std::f32::consts::PI / 5.0;
        let pt = Point::new(c + r * a.cos(), c + r * a.sin());
        p = if i == 0 { p.move_to(pt) } else { p.line_to(pt) };
    }
    p.close()
}

/// A fixed time step per frame (about 60 fps): it keeps `update` **deterministic**
/// and testable while following the subscription's rhythm.
const FRAME_DT: f32 = 1.0 / 60.0;

/// The duration of one there-and-back cycle (scale, fractional width), in seconds.
const CYCLE: f32 = 2.4;

/// The state: elapsed time, whether the animation is running, the scale slider's
/// position (`0..1`), and the number of clicks on the transformed button.
struct Showcase {
    time: f32,
    running: bool,
    scale_knob: f32,
    taps: u32,
}

impl Default for Showcase {
    fn default() -> Self {
        // `scale_knob = 1/3` corresponds to a manual scale of 1.0; see `view`.
        Self {
            time: 0.0,
            running: true,
            scale_knob: 1.0 / 3.0,
            taps: 0,
        }
    }
}

/// The messages: clock, play/pause, scale slider, transformed click.
#[derive(Clone)]
enum Msg {
    /// A frame has passed: advance the clock.
    Frame,
    /// Toggles play/pause on the automatic animation.
    ToggleRunning,
    /// A new position for the scale slider (`0..1`).
    SetKnob(f32),
    /// A click on the button placed inside a rotated `Transform`.
    Tap,
}

/// The manual scale (`0.5 → 2.0`) derived from the slider position (`0..1`).
fn knob_to_scale(knob: f32) -> f32 {
    0.5 + knob * 1.5
}

impl Application for Showcase {
    type Message = Msg;

    /// `update` is **pure**: it advances the state, with no effects and no GPU.
    fn update(&mut self, message: Msg) -> Command<Msg> {
        match message {
            Msg::Frame => {
                if self.running {
                    self.time += FRAME_DT;
                }
            }
            Msg::ToggleRunning => self.running = !self.running,
            Msg::SetKnob(v) => self.scale_knob = v.clamp(0.0, 1.0),
            Msg::Tap => self.taps += 1,
        }
        Command::none()
    }

    /// Keeps time at about 60 fps **while the animation runs**; when paused there is
    /// no subscription at all, while interactions stay event-driven.
    fn subscription(&self) -> Subscription<Msg> {
        if self.running {
            Subscription::every(Duration::from_millis(16), |_| Msg::Frame)
        } else {
            Subscription::none()
        }
    }

    /// `view` is a **pure** function of the state: it recomputes the animated values,
    /// then rebuilds the scene.
    fn view(&self, theme: &Theme, width: f32, height: f32) -> Box<dyn Widget<Msg>> {
        // A there-and-back phase `0 → 1 → 0` over one cycle, eased.
        let phase = (self.time % CYCLE) / CYCLE;
        let ping = 1.0 - (2.0 * phase - 1.0).abs();
        let eased = Curve::ease_in_out().transform(ping);

        // The animated values (Tween and sinusoids).
        let angle = self.time * 0.9; // continuous rotation, in radians
        let scale = Tween::new(1.0, 1.4).eval(eased); // a pulse
        let bob = (self.time * 2.2).sin() * 22.0; // vertical bobbing
        let drift = (self.time * 1.6).sin() * 26.0; // horizontal drift
        let squash = (self.time * 2.6).sin() * 0.35; // squash and stretch
        let width_factor = Tween::new(0.25, 1.0).eval(eased);
        let manual_scale = knob_to_scale(self.scale_knob);

        // A small rounded colour square.
        let square = |color: Color| {
            Container::<Msg>::new()
                .width(64.0)
                .height(64.0)
                .color(color)
                .radius(14.0)
        };
        // A gradient square, for the composed hero.
        let gradient_square = || {
            Container::<Msg>::new()
                .width(64.0)
                .height(64.0)
                .color(theme.primary)
                .gradient(theme.scheme.secondary, [1.0, 1.0])
                .radius(14.0)
        };
        // A tile: content centred in a fixed stage (with room to overflow), captioned.
        let tile = |inner: Box<dyn Widget<Msg>>, label: &str| {
            Flex::column()
                .gap(10.0)
                .align(Align::Center)
                .child(
                    Flex::column()
                        .width(128.0)
                        .height(128.0)
                        .justify(Justify::Center)
                        .align(Align::Center)
                        .child(inner),
                )
                .child(Text::new(label).size(12.0).color(theme.on_surface))
        };

        // Gallery 1: translation, non-uniform scale, and rotation+scale composed.
        let gallery1 = Flex::row()
            .gap(16.0)
            .align(Align::Center)
            .child(tile(
                Box::new(Transform::translate(0.0, bob).child(square(theme.primary))),
                "translate",
            ))
            .child(tile(
                Box::new(
                    Transform::scale_xy(1.0 + squash, 1.0 - squash)
                        .child(square(theme.scheme.secondary)),
                ),
                "scale_xy",
            ))
            .child(tile(
                Box::new(
                    Transform::rotate(angle)
                        .and_scale(scale)
                        .child(gradient_square()),
                ),
                "rotate + scale",
            ));

        // Gallery 2: rotation about an **offset pivot**, and translation+rotation.
        let gallery2 = Flex::row()
            .gap(16.0)
            .align(Align::Center)
            .child(tile(
                Box::new(
                    Transform::rotate_from(angle, Alignment::TOP_LEFT)
                        .child(square(theme.scheme.secondary)),
                ),
                "rotate @ corner",
            ))
            .child(tile(
                Box::new(
                    Transform::rotate(angle * 0.6)
                        .and_translate(drift, 0.0)
                        .child(square(theme.primary)),
                ),
                "translate + rotate",
            ));

        // Gallery 3: **shaped** clipping — a gradient square with **sharp** corners
        // clipped to a rounded rectangle (`ClipRRect`), then to a circle (`ClipOval`).
        // The contrast with the original sharp corners is what makes the clip visible.
        let sharp = || {
            Container::<Msg>::new()
                .width(96.0)
                .height(96.0)
                .color(theme.primary)
                .gradient(theme.scheme.secondary, [1.0, 1.0])
        };
        let gallery3 = Flex::row()
            .gap(16.0)
            .align(Align::Center)
            .child(tile(
                Box::new(ClipRRect::new(24.0).child(sharp())),
                "ClipRRect(24)",
            ))
            .child(tile(Box::new(ClipOval::new().child(sharp())), "ClipOval"))
            .child(tile(
                Box::new(ClipPath::new(star_path(96.0)).child(sharp())),
                "ClipPath (star)",
            ));

        // Gallery 4: transforms that **affect layout**. `RotatedBox` turns a text a
        // quarter turn, so its box becomes tall and narrow; `FittedBox` scales a large
        // text down to **fit** (Contain) inside a frame.
        let rotated =
            RotatedBox::new(3).child(Text::new("ROTATED").size(16.0).color(theme.on_surface));
        let fitted = Container::new()
            .width(120.0)
            .height(80.0)
            .color(theme.surface)
            .radius(8.0)
            .child(
                FittedBox::new(BoxFit::Contain)
                    .width(120.0)
                    .height(80.0)
                    .child(Text::new("Fit").size(48.0).color(theme.primary)),
            );
        let gallery4 = Flex::row()
            .gap(16.0)
            .align(Align::Center)
            .child(tile(Box::new(rotated), "RotatedBox(3)"))
            .child(tile(Box::new(fitted), "FittedBox·Contain"));

        // An **interactive** window: a grid of dots on a gradient background that the
        // user can **pan** by dragging and **zoom** with the wheel, anchored at the
        // cursor. At high zoom the content overflows and is clipped to the frame.
        let viewer_content = Container::new()
            .color(theme.surface)
            .gradient(theme.primary_container, [1.0, 1.0])
            .child(
                Flex::column()
                    .justify(Justify::Center)
                    .align(Align::Center)
                    .gap(12.0)
                    .child(
                        Flex::row()
                            .gap(12.0)
                            .child(square(theme.primary))
                            .child(square(theme.scheme.secondary))
                            .child(square(theme.primary)),
                    )
                    .child(
                        Flex::row()
                            .gap(12.0)
                            .child(square(theme.scheme.secondary))
                            .child(square(theme.primary))
                            .child(square(theme.scheme.secondary)),
                    ),
            );
        let viewer = Container::new().radius(12.0).color(theme.surface).child(
            InteractiveViewer::new()
                .width(260.0)
                .height(180.0)
                .min_scale(0.5)
                .max_scale(4.0)
                .child(viewer_content),
        );

        // Interactive: a **button clickable inside a rotated Transform**, since
        // hit-testing goes through the transform; a slider driving a scale live; and
        // a play/pause button.
        let tap_stage = Flex::column()
            .width(240.0)
            .height(96.0)
            .justify(Justify::Center)
            .align(Align::Center)
            .child(
                Transform::rotate(0.35)
                    .and_scale(1.15)
                    .child(Button::new("Tap me (rotated)").on_press(Msg::Tap)),
            );
        let tap_col = Flex::column()
            .gap(8.0)
            .align(Align::Center)
            .child(tap_stage)
            .child(
                Text::new(format!("taps: {}", self.taps))
                    .size(14.0)
                    .color(theme.on_surface),
            );

        let knob_stage = Flex::column()
            .width(200.0)
            .height(96.0)
            .justify(Justify::Center)
            .align(Align::Center)
            .child(Transform::scale(manual_scale).child(square(theme.primary)));
        let knob_col = Flex::column()
            .gap(8.0)
            .align(Align::Center)
            .child(knob_stage)
            .child(
                Slider::new(self.scale_knob)
                    .width(200.0)
                    .on_change(|v| Msg::SetKnob(v)),
            )
            .child(
                Text::new(format!("scale: {manual_scale:.2}"))
                    .size(12.0)
                    .color(theme.on_surface),
            );

        let interactive = Flex::column()
            .gap(14.0)
            .align(Align::Center)
            .child(
                Flex::row()
                    .gap(28.0)
                    .align(Align::Center)
                    .child(tap_col)
                    .child(knob_col),
            )
            .child(
                Button::new(if self.running { "pause" } else { "play" })
                    .variant(Variant::Secondary)
                    .on_press(Msg::ToggleRunning),
            );

        // AspectRatio 16:9: the box takes the width (240) and derives its height.
        let aspect = Container::new().width(240.0).child(
            AspectRatio::new(16.0 / 9.0).child(
                Container::new()
                    .flex(1.0)
                    .color(theme.primary_container)
                    .gradient(theme.scheme.secondary, [0.0, 1.0])
                    .radius(12.0),
            ),
        );

        // FractionallySizedBox: a bar whose width, a fraction of the parent, breathes.
        let bar = Container::new()
            .width(240.0)
            .height(18.0)
            .color(theme.surface)
            .radius(9.0)
            .child(
                FractionallySizedBox::new()
                    .width_factor(width_factor)
                    .child(Container::new().flex(1.0).color(theme.primary).radius(9.0)),
            );

        // The whole column, centred, with a little margin.
        let content = Flex::column()
            .width(width)
            .gap(22.0)
            .padding(24.0)
            .align(Align::Center)
            .child(
                Text::new("Transform · Clip · RotatedBox · FittedBox · InteractiveViewer")
                    .size(20.0)
                    .color(theme.on_surface),
            )
            .child(gallery1)
            .child(gallery2)
            .child(
                Text::new("ClipRRect · ClipOval · ClipPath — clipped to shape")
                    .size(13.0)
                    .color(theme.on_surface),
            )
            .child(gallery3)
            .child(
                Text::new("RotatedBox · FittedBox — transforms that change layout")
                    .size(13.0)
                    .color(theme.on_surface),
            )
            .child(gallery4)
            .child(
                Text::new("InteractiveViewer — drag to pan, wheel to zoom")
                    .size(13.0)
                    .color(theme.on_surface),
            )
            .child(viewer)
            .child(
                Text::new("Interactive — the button below is inside a rotated Transform")
                    .size(13.0)
                    .color(theme.on_surface),
            )
            .child(interactive)
            .child(
                Text::new("AspectRatio 16:9")
                    .size(13.0)
                    .color(theme.on_surface),
            )
            .child(aspect)
            .child(
                Text::new("FractionallySizedBox")
                    .size(13.0)
                    .color(theme.on_surface),
            )
            .child(bar);

        // Scrollable: the viewport fills the window through explicit width and height
        // — a default `Scroll` is only 200px tall with an automatic width — and the
        // larger content scrolls.
        Box::new(
            Container::new()
                .width(width)
                .height(height)
                .color(theme.background)
                .child(Scroll::new().width(width).height(height).child(content)),
        )
    }

    fn title(&self) -> String {
        "frus — transforms".to_string()
    }
}

// The **single** entry point: one declaration generates the desktop entry (`run()`,
// called by the binary) and the Android one (`android_main`). See `frus_shell::main!`.
frus_shell::main!(Showcase::default());

#[cfg(test)]
mod tests {
    use super::*;

    /// `update` advances the clock one fixed step per frame — pure, no GPU.
    #[test]
    fn frames_advance_the_clock() {
        let mut app = Showcase::default();
        app.update(Msg::Frame);
        app.update(Msg::Frame);
        assert!(
            (app.time - 2.0 * FRAME_DT).abs() < 1e-6,
            "temps = {}",
            app.time
        );
    }

    /// When **paused**, the clock stops and the subscription goes quiet.
    #[test]
    fn pause_stops_the_clock_and_the_subscription() {
        let mut app = Showcase::default();
        assert!(
            !app.subscription().is_empty(),
            "en marche : souscription active"
        );
        app.update(Msg::ToggleRunning);
        app.update(Msg::Frame);
        assert_eq!(app.time, 0.0, "paused: the clock is frozen");
        assert!(
            app.subscription().is_empty(),
            "en pause : plus de souscription"
        );
    }

    /// The slider drives the manual scale (`0..1 → 0.5..2.0`).
    #[test]
    fn knob_drives_the_manual_scale() {
        let mut app = Showcase::default();
        app.update(Msg::SetKnob(1.0));
        assert!((knob_to_scale(app.scale_knob) - 2.0).abs() < 1e-6);
        app.update(Msg::SetKnob(0.0));
        assert!((knob_to_scale(app.scale_knob) - 0.5).abs() < 1e-6);
    }

    /// Every click on the transformed button increments the counter.
    #[test]
    fn taps_increment() {
        let mut app = Showcase::default();
        app.update(Msg::Tap);
        app.update(Msg::Tap);
        assert_eq!(app.taps, 2);
    }

    /// A **headless** render of one frame: the `view` produces a **transformed** layer
    /// — the scene's `Transform`s — proving the showcase wires the stack end to end,
    /// with no GPU.
    #[test]
    fn renders_a_transformed_layer() {
        use frus_core::Primitive;
        use frus_widgets::{build_ui, Runtime, Size};
        let app = Showcase {
            time: 0.5,
            ..Default::default()
        };
        let theme = Theme::dark();
        let view = app.view(&theme, 400.0, 640.0);
        let rt = Runtime::default();
        let ui = build_ui(view.as_ref(), Size::new(400.0, 640.0), &rt, &theme);
        let transformed = ui.scene().primitives().iter().any(|p| {
            matches!(
                p,
                Primitive::Layer {
                    transform: Some(_),
                    ..
                }
            )
        });
        assert!(
            transformed,
            "a Transform in the scene emits a transformed layer"
        );
    }

    /// The `view` emits **shape-clipped** layers: the clip gallery produces a
    /// `ClipShape::RRect` **and** a `ClipShape::Oval`, proving the showcase wires
    /// clipping end to end.
    #[test]
    fn renders_clip_shapes() {
        use frus_core::{BorderRadius, ClipShape, Primitive};
        use frus_widgets::{build_ui, Runtime, Size};
        let app = Showcase::default();
        let theme = Theme::dark();
        let view = app.view(&theme, 500.0, 900.0);
        let rt = Runtime::default();
        let ui = build_ui(view.as_ref(), Size::new(500.0, 900.0), &rt, &theme);
        // Recursively collect the clip shapes, since layers can nest.
        fn shapes(prims: &[Primitive], out: &mut Vec<ClipShape>) {
            for p in prims {
                if let Primitive::Layer {
                    clip_shape,
                    primitives,
                    ..
                } = p
                {
                    out.push(clip_shape.clone());
                    shapes(primitives, out);
                }
            }
        }
        let mut found = Vec::new();
        shapes(ui.scene().primitives(), &mut found);
        assert!(
            found.contains(&ClipShape::RRect(BorderRadius::uniform(24.0))),
            "ClipRRect(24) rendu : {found:?}"
        );
        assert!(
            found.contains(&ClipShape::Oval),
            "ClipOval rendu : {found:?}"
        );
        assert!(
            found.iter().any(|s| matches!(s, ClipShape::Path(_))),
            "ClipPath (star) rendered: {found:?}"
        );
    }

    /// A blank-page guard: the content must be **genuinely sized** and placed
    /// **inside** the window — at least one wide rectangle at a visible position,
    /// which shows the `Scroll` viewport fills the window instead of collapsing.
    #[test]
    fn content_is_laid_out_within_the_window() {
        use frus_core::Primitive;
        use frus_widgets::{build_ui, Runtime, Size};
        let (w, h) = (1000.0_f32, 800.0_f32);
        let app = Showcase::default();
        let view = app.view(&Theme::dark(), w, h);
        let rt = Runtime::default();
        let ui = build_ui(view.as_ref(), Size::new(w, h), &rt, &Theme::dark());
        let wide_on_screen = ui.scene().primitives().iter().any(|p| match p {
            Primitive::Rect { rect, .. } => {
                rect.width > 100.0 && rect.x >= -1.0 && rect.x < w && rect.y >= -1.0 && rect.y < h
            }
            _ => false,
        });
        assert!(
            wide_on_screen,
            "no wide content visible: has the viewport collapsed?"
        );
    }
}
