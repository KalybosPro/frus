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

/// A 2×1 PNG: a red pixel, then a blue one.
const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAIAAAABCAYAAAD0In+KAAAADklEQVR4nGP4z8AAQv8BD/kD/YURmXYAAAAASUVORK5CYII=";

/// A 20-unit SVG — a circle with a gradient, a green square — as a `data:` address.
const NESTED: &str = "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIyMCIgaGVpZ2h0PSIyMCIgdmlld0JveD0iMCAwIDIwIDIwIj48ZGVmcz48bGluZWFyR3JhZGllbnQgaWQ9ImciPjxzdG9wIG9mZnNldD0iMCIgc3RvcC1jb2xvcj0iI2U1MzkzNSIvPjxzdG9wIG9mZnNldD0iMSIgc3RvcC1jb2xvcj0iIzFlODhlNSIvPjwvbGluZWFyR3JhZGllbnQ+PC9kZWZzPjxjaXJjbGUgY3g9IjEwIiBjeT0iMTAiIHI9IjgiIGZpbGw9InVybCgjZykiLz48cmVjdCB4PSIyIiB5PSIyIiB3aWR0aD0iNiIgaGVpZ2h0PSI2IiBmaWxsPSIjNDNhMDQ3Ii8+PC9zdmc+";

#[test]
fn an_embedded_svg_is_drawn_like_the_reference_straight_and_turned() {
    matches_the_reference(
        "nested",
        &format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100">
              <image x="5" y="5" width="40" height="40" href="{NESTED}"/>
              <image x="50" y="40" width="40" height="40" transform="rotate(30 70 60)"
                     href="{NESTED}"/></svg>"##
        ),
    );
}

/// The reference is built without its bitmap decoder, so a bitmap is checked by its colours:
/// red on the left of the picture, blue on its right, wherever the picture turns it.
#[test]
fn an_embedded_bitmap_is_drawn_straight_and_turned() {
    let Some(ours) = painted(&format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100">
          <image x="10" y="10" width="40" height="20" href="data:image/png;base64,{PNG}"/>
          <image x="50" y="50" width="40" height="20" transform="rotate(90 50 50)"
                 href="data:image/png;base64,{PNG}"/></svg>"##
    )) else {
        return;
    };
    let at = |x: f32, y: f32| {
        let scale = SIZE as f32 / 100.0;
        ours.pixel((x * scale) as u32, (y * scale) as u32)
    };
    let red = |p: [u8; 4]| p[0] > 200 && p[1] < 40 && p[2] < 40;
    let blue = |p: [u8; 4]| p[2] > 200 && p[0] < 40 && p[1] < 40;
    let white = |p: [u8; 4]| p[0] > 245 && p[1] > 245 && p[2] > 245;
    assert!(red(at(15.0, 20.0)), "straight, left: {:?}", at(15.0, 20.0));
    assert!(
        blue(at(45.0, 20.0)),
        "straight, right: {:?}",
        at(45.0, 20.0)
    );
    // Turned a quarter about its corner: it spans x 30..50, y 50..90, its left at the top.
    assert!(red(at(40.0, 55.0)), "turned, left: {:?}", at(40.0, 55.0));
    assert!(blue(at(40.0, 85.0)), "turned, right: {:?}", at(40.0, 85.0));
    assert!(
        white(at(25.0, 70.0)) && white(at(55.0, 70.0)),
        "nothing beside it"
    );
    assert!(white(at(40.0, 95.0)), "nothing past it");
}
