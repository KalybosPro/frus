//! **A colour asked for is the colour painted** — on every surface that can paint one.
//!
//! Three milestones in a row were colours that did not survive the trip to the screen: a
//! blend space (328), the disabled tokens (329), and every glyph in the framework painted
//! at its linearised value (330). Each was found by accident — a photograph of a phone,
//! an arithmetic that stopped agreeing with a picture — and each would have been caught
//! here in a second.
//!
//! The failure mode is always the same shape: sRGB and linear light are one function
//! apart, and applying that function once too often, or once too few, still produces a
//! plausible picture. The scene primitive is right, the widget test passes, the golden
//! looks fine to anyone who has not seen the correct one. So this asks the only question
//! that settles it — *render a known colour and read the pixel back*.
//!
//! Each surface reaches the GPU through its own path and does its own conversion:
//!
//! | surface | who converts |
//! |---|---|
//! | quads (`Primitive::Rect`) | `quad.wgsl`'s `srgb_to_linear` |
//! | text (`Primitive::Text`) | glyphon's shader |
//! | paths (`Primitive::Path`) | `path.wgsl`'s `srgb_to_linear` |
//! | images (`Primitive::Image`) | `image.wgsl`, on the *tint* |
//!
//! They differ, so each needs its own case. Milestone 330 was exactly the slip of handing
//! one path a colour prepared for another.
//!
//! What this deliberately does **not** cover is translucency: an alpha still blends in
//! linear light, which `blending.rs` pins separately and the roadmap tracks. Every colour
//! here is opaque.

use frus_core::{Color, ImageData, Path, Point, Rect, Scene};
use frus_test::render_scene;

/// The colour under test. Mid-range on every channel and different on each, so that a
/// stray conversion moves it measurably and a channel swap is visible too. Near the
/// extremes both transfer functions flatten out and a slip hides.
const ASKED: Color = Color {
    r: 0.4,
    g: 0.6,
    b: 0.25,
    a: 1.0,
};

/// `ASKED` as the eight-bit values a screenshot shows: (102, 153, 64).
fn asked_u8() -> [u8; 3] {
    [
        (ASKED.r * 255.0).round() as u8,
        (ASKED.g * 255.0).round() as u8,
        (ASKED.b * 255.0).round() as u8,
    ]
}

/// What the pixel would be if one sRGB→linear conversion escaped — the shape of every one
/// of the three bugs. Reported alongside a failure so the diagnosis comes with it.
fn linearised() -> [u8; 3] {
    let f = |v: f32| {
        let lin = if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        };
        (lin * 255.0).round() as u8
    };
    [f(ASKED.r), f(ASKED.g), f(ASKED.b)]
}

/// And if one escaped the other way.
fn encoded_twice() -> [u8; 3] {
    let f = |v: f32| {
        let s = if v <= 0.0031308 {
            v * 12.92
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        };
        (s * 255.0).round() as u8
    };
    [f(ASKED.r), f(ASKED.g), f(ASKED.b)]
}

/// Asserts `painted` is `ASKED`, and says which conversion slipped when it is not.
fn assert_asked(surface: &str, painted: [u8; 3]) {
    let want = asked_u8();
    if painted == want {
        return;
    }
    let diagnosis = if painted == linearised() {
        " — one sRGB→linear conversion too many, the shape milestone 330 had"
    } else if painted == encoded_twice() {
        " — that is `ASKED` encoded twice"
    } else {
        ""
    };
    panic!(
        "{surface}: asked for {want:?}, painted {painted:?}{diagnosis}\n\
         (linearised would be {:?}, encoded twice {:?})",
        linearised(),
        encoded_twice()
    );
}

/// The pixel furthest from `clear` in the frame — the core of whatever was drawn. For a
/// filled shape that is any interior pixel; for a glyph it is the middle of a stroke,
/// where coverage reaches one and no antialiasing is mixed in.
fn strongest(shot: &frus_test::Snapshot, clear: [u8; 3]) -> [u8; 3] {
    let mut best = ([0u8; 3], -1i32);
    for y in 0..shot.height {
        for x in 0..shot.width {
            let p = shot.pixel(x, y);
            let d = (0..3)
                .map(|i| (p[i] as i32 - clear[i] as i32).abs())
                .sum::<i32>();
            if d > best.1 {
                best = ([p[0], p[1], p[2]], d);
            }
        }
    }
    best.0
}

/// A dark clear colour, so that anything drawn stands well clear of it.
const CLEAR: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};

#[test]
fn a_quad_is_painted_the_colour_it_asked_for() {
    let mut scene = Scene::new();
    scene.draw_rect(
        Rect::new(10.0, 10.0, 60.0, 60.0),
        ASKED,
        0.0,
        0.0,
        Color::TRANSPARENT,
    );
    let Some(shot) = render_scene(&scene, 80, 80, CLEAR) else {
        return; // no GPU here; the claim is about the pipeline
    };
    let p = shot.pixel(40, 40);
    assert_asked("quad", [p[0], p[1], p[2]]);
}

#[test]
fn a_path_is_painted_the_colour_it_asked_for() {
    let mut scene = Scene::new();
    scene.fill_path(&Path::rect(Rect::new(10.0, 10.0, 60.0, 60.0)), ASKED);
    let Some(shot) = render_scene(&scene, 80, 80, CLEAR) else {
        return;
    };
    let p = shot.pixel(40, 40);
    assert_asked("filled path", [p[0], p[1], p[2]]);
}

#[test]
fn a_glyph_is_painted_the_colour_it_asked_for() {
    // Large and heavy, so a stroke has interior pixels at full coverage. A thin glyph is
    // all antialiasing and never reaches its own colour.
    let mut scene = Scene::new();
    scene.text(Point::new(4.0, 4.0), "MMMM", 72.0, ASKED);
    let Some(shot) = render_scene(&scene, 260, 110, CLEAR) else {
        return;
    };
    assert_asked("glyph", strongest(&shot, [0, 0, 0]));
}

#[test]
fn an_image_tint_is_painted_the_colour_it_asked_for() {
    // A white image takes the tint unchanged: the tint is multiplicative, so white × tint
    // is the tint, and any conversion applied to it shows up neat.
    let white = ImageData::from_rgba(2, 2, vec![255u8; 16]).into_handle();
    let mut scene = Scene::new();
    let dst = Rect::new(10.0, 10.0, 60.0, 60.0);
    scene.draw_image(&white, dst, Rect::new(0.0, 0.0, 1.0, 1.0), ASKED);
    let Some(shot) = render_scene(&scene, 80, 80, CLEAR) else {
        return;
    };
    let p = shot.pixel(40, 40);
    assert_asked("image tint", [p[0], p[1], p[2]]);
}
