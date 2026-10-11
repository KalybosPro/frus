//! A layer that **blends with what is under it** (milestone 646), rendered.
//!
//! Every mode is checked against its formula, worked here on the CPU in the space colours
//! are written in, sRGB, as the reference's are. Then what the blend must leave alone:
//! the frame outside the layer's clip, and, for a layer inside a group, everything that
//! is not the group's own drawing.

use frus_core::{BlendMode, ClipShape, Color, Primitive, Rect, Scene};
use frus_test::render_scene;

/// What is under the layer, and what the layer paints: far enough apart that every mode
/// tells them apart.
const UNDER: [f32; 3] = [0.3, 0.6, 0.5];
const OVER: [f32; 3] = [0.8, 0.2, 0.4];

/// A second pair, at the formulas' edges: a dark destination under a light source, and
/// channels at nought and one, where the dodge, the burn and the soft light branch.
const DARK_UNDER: [f32; 3] = [0.1, 1.0, 0.2];
const LIGHT_OVER: [f32; 3] = [0.95, 0.0, 1.0];

fn lum(c: [f32; 3]) -> f32 {
    0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
}

fn clip_color(c: [f32; 3]) -> [f32; 3] {
    let l = lum(c);
    let n = c[0].min(c[1]).min(c[2]);
    let x = c[0].max(c[1]).max(c[2]);
    let mut o = c;
    if n < 0.0 {
        o = o.map(|v| l + (v - l) * l / (l - n));
    }
    if x > 1.0 {
        o = o.map(|v| l + (v - l) * (1.0 - l) / (x - l));
    }
    o
}

fn set_lum(c: [f32; 3], l: f32) -> [f32; 3] {
    let d = l - lum(c);
    clip_color(c.map(|v| v + d))
}

fn sat(c: [f32; 3]) -> f32 {
    c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])
}

fn set_sat(c: [f32; 3], s: f32) -> [f32; 3] {
    let lo = c[0].min(c[1]).min(c[2]);
    let hi = c[0].max(c[1]).max(c[2]);
    if hi > lo {
        c.map(|v| (v - lo) * s / (hi - lo))
    } else {
        [0.0; 3]
    }
}

/// One channel of a separable mode.
fn separable(mode: BlendMode, s: f32, d: f32) -> f32 {
    match mode {
        BlendMode::Multiply => s * d,
        BlendMode::Screen => s + d - s * d,
        BlendMode::Overlay => separable(BlendMode::HardLight, d, s),
        BlendMode::Darken => s.min(d),
        BlendMode::Lighten => s.max(d),
        BlendMode::ColorDodge => {
            if d <= 0.0 {
                0.0
            } else if s >= 1.0 {
                1.0
            } else {
                (d / (1.0 - s)).min(1.0)
            }
        }
        BlendMode::ColorBurn => {
            if d >= 1.0 {
                1.0
            } else if s <= 0.0 {
                0.0
            } else {
                1.0 - ((1.0 - d) / s).min(1.0)
            }
        }
        BlendMode::HardLight => {
            if s <= 0.5 {
                2.0 * s * d
            } else {
                1.0 - 2.0 * (1.0 - s) * (1.0 - d)
            }
        }
        BlendMode::SoftLight => {
            if s <= 0.5 {
                d - (1.0 - 2.0 * s) * d * (1.0 - d)
            } else {
                let dd = if d <= 0.25 {
                    ((16.0 * d - 12.0) * d + 4.0) * d
                } else {
                    d.sqrt()
                };
                d + (2.0 * s - 1.0) * (dd - d)
            }
        }
        BlendMode::Difference => (s - d).abs(),
        BlendMode::Exclusion => s + d - 2.0 * s * d,
        other => unreachable!("{other:?} is not separable"),
    }
}

/// `s` on `d`, both opaque, by `mode`: what the frame should hold.
fn expected(mode: BlendMode, s: [f32; 3], d: [f32; 3]) -> [f32; 4] {
    let opaque = |c: [f32; 3]| [c[0], c[1], c[2], 1.0];
    match mode {
        BlendMode::Src | BlendMode::SrcOver | BlendMode::SrcIn | BlendMode::SrcAtop => opaque(s),
        BlendMode::Dst | BlendMode::DstOver | BlendMode::DstIn | BlendMode::DstAtop => opaque(d),
        BlendMode::SrcOut | BlendMode::DstOut | BlendMode::Xor | BlendMode::Clear => [0.0; 4],
        BlendMode::Plus => opaque([0, 1, 2].map(|i| (s[i] + d[i]).min(1.0))),
        BlendMode::Modulate => opaque([0, 1, 2].map(|i| s[i] * d[i])),
        BlendMode::Hue => opaque(set_lum(set_sat(s, sat(d)), lum(d))),
        BlendMode::Saturation => opaque(set_lum(set_sat(d, sat(s)), lum(d))),
        BlendMode::Color => opaque(set_lum(s, lum(d))),
        BlendMode::Luminosity => opaque(set_lum(d, lum(s))),
        separable_mode => opaque([0, 1, 2].map(|i| separable(separable_mode, s[i], d[i]))),
    }
}

const ALL: [BlendMode; 29] = [
    BlendMode::Src,
    BlendMode::Dst,
    BlendMode::SrcOver,
    BlendMode::DstOver,
    BlendMode::SrcIn,
    BlendMode::DstIn,
    BlendMode::SrcOut,
    BlendMode::DstOut,
    BlendMode::SrcAtop,
    BlendMode::DstAtop,
    BlendMode::Xor,
    BlendMode::Plus,
    BlendMode::Modulate,
    BlendMode::Multiply,
    BlendMode::Screen,
    BlendMode::Overlay,
    BlendMode::Darken,
    BlendMode::Lighten,
    BlendMode::Clear,
    BlendMode::ColorDodge,
    BlendMode::ColorBurn,
    BlendMode::HardLight,
    BlendMode::SoftLight,
    BlendMode::Difference,
    BlendMode::Exclusion,
    BlendMode::Hue,
    BlendMode::Saturation,
    BlendMode::Color,
    BlendMode::Luminosity,
];

fn color(c: [f32; 3]) -> Color {
    Color::rgb(c[0], c[1], c[2])
}

/// A pixel, premultiplied, as `0..=255`.
fn bytes(c: [f32; 4]) -> [i32; 4] {
    c.map(|v| (v * 255.0).round() as i32)
}

fn close(got: [u8; 4], want: [i32; 4]) -> bool {
    got.iter()
        .zip(want)
        .all(|(&g, w)| (g as i32 - w).abs() <= 2)
}

/// **Each mode meets what is under it by its formula**, inside the layer's clip, and
/// leaves the frame alone outside it.
#[test]
fn every_mode_meets_what_is_under_it_as_written() {
    let mut wrong = Vec::new();
    for (over, under) in [(OVER, UNDER), (LIGHT_OVER, DARK_UNDER)] {
        for mode in ALL {
            let mut scene = Scene::new();
            scene.fill_rect(Rect::new(0.0, 0.0, 40.0, 40.0), color(under));
            scene.set_clip(Rect::new(10.0, 10.0, 20.0, 20.0));
            scene.blended(mode, |s| {
                s.fill_rect(Rect::new(0.0, 0.0, 40.0, 40.0), color(over));
            });
            let Some(shot) = render_scene(&scene, 40, 40, Color::TRANSPARENT) else {
                eprintln!("no GPU adapter available: test skipped");
                return;
            };
            let inside = shot.pixel(20, 20);
            let want = bytes(expected(mode, over, under));
            if !close(inside, want) {
                wrong.push(format!(
                    "{mode:?} of {over:?} on {under:?}: painted {inside:?}, wanted {want:?}"
                ));
            }
            let outside = shot.pixel(4, 4);
            let kept = bytes([under[0], under[1], under[2], 1.0]);
            if !close(outside, kept) {
                wrong.push(format!(
                    "{mode:?} outside its clip: {outside:?}, wanted {kept:?}"
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// **A layer over a backdrop is painted over it**: the frame is cut once at a layer, for
/// its backdrop, so a blend asked for beside one is not applied — and the layer is not
/// blended with a stand-in for what is under it either.
#[test]
fn a_layer_over_a_backdrop_is_painted_over_it() {
    let mut scene = Scene::new();
    scene.fill_rect(Rect::new(0.0, 0.0, 40.0, 40.0), color(UNDER));
    scene.set_clip(Rect::new(0.0, 0.0, 40.0, 40.0));
    scene.push_primitive(Primitive::Layer {
        primitives: vec![Primitive::Rect {
            rect: Rect::new(0.0, 0.0, 40.0, 40.0),
            color: color(OVER),
            color2: color(OVER),
            gradient_dir: [0.0, 0.0],
            radius: frus_core::BorderRadius::ZERO,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            blur: 0.0,
            clip: Rect::UNBOUNDED,
            owner: 0,
            shader: None,
            blur_style: frus_core::BlurStyle::Normal,
        }],
        opacity: 1.0,
        clip: Rect::new(0.0, 0.0, 40.0, 40.0),
        clip_shape: ClipShape::Rect,
        transform: None,
        filter: frus_core::LayerFilter {
            backdrop: Some(frus_core::Backdrop::blur(2.0)),
            blend: Some(BlendMode::Difference),
            ..frus_core::LayerFilter::NONE
        },
        owner: 0,
    });
    let Some(shot) = render_scene(&scene, 40, 40, Color::TRANSPARENT) else {
        return;
    };
    let got = shot.pixel(20, 20);
    assert!(
        close(got, bytes([OVER[0], OVER[1], OVER[2], 1.0])),
        "the layer's own colour, painted over: {got:?}"
    );
}

/// **A translucent layer blends by its own coverage**: half a multiply darkens half as
/// much, the formula's own `(1 - αs)·d + αs·B` with nothing under it transparent.
#[test]
fn a_translucent_layer_blends_by_its_alpha() {
    let mut scene = Scene::new();
    scene.fill_rect(Rect::new(0.0, 0.0, 40.0, 40.0), color(UNDER));
    scene.blended(BlendMode::Multiply, |s| {
        s.fill_rect(Rect::new(0.0, 0.0, 40.0, 40.0), color(OVER).with_alpha(0.5));
    });
    let Some(shot) = render_scene(&scene, 40, 40, Color::TRANSPARENT) else {
        return;
    };
    let want = [0, 1, 2].map(|i| 0.5 * UNDER[i] + 0.5 * OVER[i] * UNDER[i]);
    let got = shot.pixel(20, 20);
    assert!(
        close(got, bytes([want[0], want[1], want[2], 1.0])),
        "painted {got:?}, wanted {want:?}"
    );
}

/// **The clip's shape is the blend's coverage**: a circle cleared out of the frame leaves
/// its corners as they were.
#[test]
fn the_clip_shape_is_where_the_blend_applies() {
    let mut scene = Scene::new();
    scene.fill_rect(Rect::new(0.0, 0.0, 40.0, 40.0), color(UNDER));
    scene.set_clip(Rect::new(0.0, 0.0, 40.0, 40.0));
    scene.blended_in(BlendMode::Clear, ClipShape::Oval, |s| {
        s.fill_rect(Rect::new(0.0, 0.0, 40.0, 40.0), color(OVER));
    });
    let Some(shot) = render_scene(&scene, 40, 40, Color::TRANSPARENT) else {
        return;
    };
    assert_eq!(shot.pixel(20, 20)[3], 0, "cleared in the circle");
    assert!(
        close(shot.pixel(1, 1), bytes([UNDER[0], UNDER[1], UNDER[2], 1.0])),
        "kept in its corner: {:?}",
        shot.pixel(1, 1)
    );
}

/// **A layer blending inside a group meets the group's drawing**, not the frame's: a
/// multiply inside a group that drew red over a blue frame darkens the red, and where the
/// group drew nothing it has nothing to darken.
#[test]
fn a_blend_inside_a_group_meets_the_group() {
    let red = Color::rgb(1.0, 0.0, 0.0);
    let grey = Color::rgb(0.5, 0.5, 0.5);
    let blue = Color::rgb(0.0, 0.0, 1.0);
    let mut scene = Scene::new();
    scene.fill_rect(Rect::new(0.0, 0.0, 60.0, 40.0), blue);
    // A group in part of the frame, so it has a region of its own.
    scene.set_clip(Rect::new(10.0, 0.0, 40.0, 40.0));
    scene.layer(1.0, |group| {
        group.fill_rect(Rect::new(10.0, 0.0, 20.0, 40.0), red);
        group.blended(BlendMode::Multiply, |s| {
            s.fill_rect(Rect::new(10.0, 0.0, 40.0, 40.0), grey);
        });
    });
    let Some(shot) = render_scene(&scene, 60, 40, Color::TRANSPARENT) else {
        return;
    };
    let darkened = shot.pixel(20, 20);
    assert!(
        close(darkened, [128, 0, 0, 255]),
        "the group's red, multiplied: {darkened:?}"
    );
    // Where the group drew nothing, the multiply of nothing is the grey itself over the
    // frame's blue — the formula with a transparent destination.
    let alone = shot.pixel(40, 20);
    assert!(
        close(alone, [128, 128, 128, 255]),
        "grey over the frame, not multiplied into it: {alone:?}"
    );
    assert!(
        close(shot.pixel(55, 20), [0, 0, 255, 255]),
        "the frame outside the group"
    );
    let nested = scene.primitives().iter().any(|p| match p {
        Primitive::Layer { primitives, .. } => primitives
            .iter()
            .any(|q| matches!(q, Primitive::Layer { filter, .. } if filter.blend.is_some())),
        _ => false,
    });
    assert!(nested, "the blend is inside the group");
}
