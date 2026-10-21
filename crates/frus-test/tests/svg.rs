//! **SVG pictures as painted, against a reference rasteriser** (milestone 690).
//!
//! Each picture is drawn by frus — parsed by `frus-svg`, painted as paths, rendered on the
//! GPU — and by resvg, the reference renderer of the parser frus uses, onto the same white
//! canvas. The two must agree everywhere but along edges, where two antialiasing schemes
//! never quite do: a gradient aimed the wrong way, a stroke join missing, a clip ignored or
//! an opacity applied twice moves far more pixels than that.

use frus_core::{Alignment, AlignmentGeometry, BoxFit, Color, Rect, Scene};
use frus_svg::Svg;
use frus_test::{render_scene, Snapshot};
use resvg::{tiny_skia, usvg};

const SIZE: u32 = 160;

/// resvg's picture of `source` on white, as a snapshot.
fn reference(source: &str) -> Snapshot {
    let tree = usvg::Tree::from_data(source.as_bytes(), &usvg::Options::default()).unwrap();
    let mut pixmap = tiny_skia::Pixmap::new(SIZE, SIZE).unwrap();
    pixmap.fill(tiny_skia::Color::WHITE);
    let scale = SIZE as f32 / tree.size().width();
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    // Opaque everywhere (the canvas is), so premultiplied is plain.
    Snapshot {
        width: SIZE,
        height: SIZE,
        rgba: pixmap.data().to_vec(),
    }
}

/// frus's picture of `source` on white, or `None` without a GPU.
fn painted(source: &str) -> Option<Snapshot> {
    let svg = Svg::parse(source.as_bytes()).unwrap();
    assert!(svg.unsupported().is_empty(), "{:?}", svg.unsupported());
    let mut scene = Scene::new();
    scene.set_clip(Rect::new(0.0, 0.0, SIZE as f32, SIZE as f32));
    svg.paint(
        &mut scene,
        Rect::new(0.0, 0.0, SIZE as f32, SIZE as f32),
        BoxFit::Contain,
        AlignmentGeometry::Physical(Alignment::CENTER),
        None,
        1.0,
    );
    render_scene(&scene, SIZE, SIZE, Color::WHITE)
}

/// Fewer than 2 % of the pixels may differ by more than 40 levels on a channel: the
/// edges, where frus's multisampling and resvg's coverage disagree by a step.
fn matches_the_reference(name: &str, source: &str) {
    let Some(ours) = painted(source) else {
        return;
    };
    let theirs = reference(source);
    let off = ours.diff_count(&theirs, 40);
    let limit = (SIZE * SIZE) as usize / 50;
    if off > limit {
        let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("svg");
        ours.write_png(dir.join(format!("{name}.frus.png")));
        theirs.write_png(dir.join(format!("{name}.resvg.png")));
        panic!(
            "{name}: {off} pixels differ (at most {limit}); both pictures are in {}",
            dir.display()
        );
    }
}

#[test]
fn shapes_and_fill_rules() {
    matches_the_reference(
        "shapes",
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
          <rect x="5" y="5" width="40" height="30" rx="6" fill="#e53935"/>
          <circle cx="72" cy="22" r="18" fill="#43a047"/>
          <ellipse cx="25" cy="70" rx="20" ry="12" fill="#1e88e5"/>
          <path d="M55 50 L95 95 L55 95 Z M65 70 L85 92 L65 92 Z" fill="#8e24aa" fill-rule="evenodd"/>
          <path d="M5 95 L25 50 L45 95 L5 65 L45 65 Z" fill="#fb8c00" fill-rule="nonzero"/>
        </svg>"##,
    );
}

#[test]
fn strokes_with_caps_joins_and_dashes() {
    matches_the_reference(
        "strokes",
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
          <polyline points="10,20 40,10 70,30 90,10" fill="none" stroke="#3949ab" stroke-width="6"
                    stroke-linejoin="round" stroke-linecap="round"/>
          <polyline points="10,50 40,40 70,60 90,40" fill="none" stroke="#00897b" stroke-width="6"
                    stroke-linejoin="miter"/>
          <polyline points="10,80 40,70 70,90 90,70" fill="none" stroke="#c0ca33" stroke-width="6"
                    stroke-linejoin="bevel" stroke-dasharray="10 5"/>
        </svg>"##,
    );
}

#[test]
fn gradients_with_stops_spread_and_transforms() {
    matches_the_reference(
        "gradients",
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
          <defs>
            <linearGradient id="a" x1="0" y1="0" x2="1" y2="0.3">
              <stop offset="0" stop-color="#d81b60"/><stop offset="0.5" stop-color="#fdd835"/>
              <stop offset="1" stop-color="#1e88e5"/></linearGradient>
            <radialGradient id="b" cx="0.5" cy="0.5" r="0.5" fx="0.3" fy="0.3">
              <stop offset="0" stop-color="#ffffff"/><stop offset="1" stop-color="#2e7d32"/></radialGradient>
            <linearGradient id="c" x1="0" y1="0" x2="10" y2="0" gradientUnits="userSpaceOnUse"
                            spreadMethod="reflect" gradientTransform="rotate(30)">
              <stop offset="0" stop-color="#6a1b9a"/><stop offset="1" stop-color="#ffb300"/></linearGradient>
          </defs>
          <rect x="5" y="5" width="90" height="25" fill="url(#a)"/>
          <circle cx="30" cy="62" r="25" fill="url(#b)"/>
          <rect x="60" y="40" width="35" height="55" fill="url(#c)"/>
        </svg>"##,
    );
}

#[test]
fn transforms_and_clips() {
    matches_the_reference(
        "groups",
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
          <defs><clipPath id="c"><circle cx="70" cy="70" r="25"/></clipPath></defs>
          <g transform="translate(10 55) rotate(-15) scale(1.2 0.8)">
            <rect width="30" height="20" fill="#43a047"/>
          </g>
          <g clip-path="url(#c)">
            <rect x="40" y="40" width="60" height="60" fill="#fb8c00"/>
            <rect x="40" y="65" width="60" height="10" fill="#3949ab"/>
          </g>
        </svg>"##,
    );
}

/// **Group opacity is one layer, blended as frus blends** — in linear light, where SVG
/// renderers blend in sRGB. Two overlapping squares at half opacity are faded as one (the
/// overlap shows the top square alone, not the two mixed), and a half-faded red over
/// white is the linear mix: paler than resvg's by a few dozen levels on the darker
/// channels. That difference is the framework's (milestone 437 and the roadmap), not the
/// picture's, so it is pinned here rather than compared.
#[test]
fn group_opacity_fades_the_group_as_one_in_linear_light() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
        <g opacity="0.5">
          <rect x="5" y="5" width="40" height="40" fill="#e53935"/>
          <rect x="25" y="25" width="40" height="40" fill="#1e88e5"/>
        </g></svg>"##;
    let Some(ours) = painted(source) else {
        return;
    };
    let lin = |v: f32| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let srgb = |v: f32| {
        if v <= 0.0031308 {
            v * 12.92
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        }
    };
    let over_white =
        |c: [u8; 3]| c.map(|v| (srgb(0.5 * lin(v as f32 / 255.0) + 0.5) * 255.0).round() as i32);
    let close =
        |got: [u8; 4], want: [i32; 3]| (0..3).all(|i| (i32::from(got[i]) - want[i]).abs() <= 3);
    // The red square alone (svg 15, 15 → 24, 24 on the canvas).
    let red = ours.pixel(24, 24);
    assert!(
        close(red, over_white([0xe5, 0x39, 0x35])),
        "red at half: {red:?}"
    );
    // Where they overlap, the blue square alone: the group was faded as one.
    let overlap = ours.pixel(56, 56);
    assert!(
        close(overlap, over_white([0x1e, 0x88, 0xe5])),
        "overlap: {overlap:?}"
    );
}
