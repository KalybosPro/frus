//! **Pixel-level** tests of a layer's shaped clip
//! ([`frus_core::Primitive::Layer`] plus [`frus_core::ClipShape`]): a solid layer is
//! composited by the GPU with a **rounded** or **elliptical** clip, and we check that
//! the pixels **outside the shape** are erased while the core stays painted. This is
//! the piece `frus-widgets`'s tests do not cover: those prove the right
//! [`frus_core::ClipShape`] is emitted, these prove the compositing shader
//! (`composite.wgsl`) **applies** it, through SDF coverage.
//!
//! With no GPU adapter `render_scene` returns `None` and the test skips itself.
//!
//! Convention: origin at the top left, y pointing down.

use frus_core::{BorderRadius, ClipShape, Color, LayerFilter, Path, Point, Primitive, Rect, Scene};
use frus_test::{render_scene, Snapshot};

/// A scene: a solid red `inner` rectangle wrapped in a layer clipped to `shape`,
/// whose box is `inner` itself.
fn clipped_layer(inner: Rect, shape: ClipShape) -> Scene {
    let mut content = Scene::new();
    content.fill_rect(inner, Color::rgb(1.0, 0.0, 0.0));
    let primitives = content.split_off(0);
    let mut scene = Scene::new();
    scene.push_primitive(Primitive::Layer {
        primitives,
        opacity: 1.0,
        clip: inner,
        clip_shape: shape,
        transform: None,
        filter: LayerFilter::NONE,
        owner: 0,
    });
    scene
}

fn is_red(px: [u8; 4]) -> bool {
    px[0] > 200 && px[1] < 60 && px[2] < 60
}

fn is_clear(px: [u8; 4]) -> bool {
    px[0] < 60 && px[1] < 60 && px[2] < 60
}

/// With no adapter we skip cleanly, as the other render tests do.
fn render(scene: &Scene) -> Option<Snapshot> {
    let frame = render_scene(scene, 64, 64, Color::BLACK);
    if frame.is_none() {
        eprintln!("no GPU adapter available: test skipped");
    }
    frame
}

/// **Rounded corners.** A solid square clipped with a large radius loses its
/// corners: the centre stays painted, but the square's top-left corner is erased.
#[test]
fn rrect_clip_rounds_off_the_corners() {
    // A centred 40×40 square: x, y ∈ [12, 52].
    let sq = Rect::new(12.0, 12.0, 40.0, 40.0);
    let m = ClipShape::RRect(BorderRadius::uniform(16.0));
    let Some(frame) = render(&clipped_layer(sq, m)) else {
        return;
    };

    // The core and the edge midpoints stay painted.
    assert!(is_red(frame.pixel(32, 32)), "centre → red");
    assert!(is_red(frame.pixel(32, 14)), "middle of the top edge → red");
    assert!(is_red(frame.pixel(14, 32)), "middle of the left edge → red");
    // The corners — inside the square but outside the radius — are erased.
    assert!(
        is_clear(frame.pixel(14, 14)),
        "top-left corner → erased by the rounding"
    );
    assert!(
        is_clear(frame.pixel(50, 50)),
        "bottom-right corner → erased by the rounding"
    );
}

/// **Per-corner radius.** Only the **top-left** corner is rounded, with radius 16;
/// the other three stay **square**. The top-left corner is erased, the rest kept.
#[test]
fn rrect_clip_rounds_only_the_specified_corner() {
    let sq = Rect::new(12.0, 12.0, 40.0, 40.0); // x, y ∈ [12, 52]
    let br = BorderRadius {
        top_left: 16.0,
        top_right: 0.0,
        bottom_right: 0.0,
        bottom_left: 0.0,
    };
    let Some(frame) = render(&clipped_layer(sq, ClipShape::RRect(br))) else {
        return;
    };

    assert!(
        is_clear(frame.pixel(15, 15)),
        "top-left corner → rounded, so erased"
    );
    assert!(
        is_red(frame.pixel(49, 15)),
        "top-right corner → square, so kept"
    );
    assert!(
        is_red(frame.pixel(15, 49)),
        "bottom-left corner → square, so kept"
    );
    assert!(
        is_red(frame.pixel(49, 49)),
        "bottom-right corner → square, so kept"
    );
}

/// **Ellipse.** A solid square clipped to an oval keeps its centre but loses all
/// four corners, which fall outside the inscribed disc.
#[test]
fn oval_clip_keeps_the_inscribed_disc() {
    let sq = Rect::new(12.0, 12.0, 40.0, 40.0); // centre (32, 32), radius 20
    let Some(frame) = render(&clipped_layer(sq, ClipShape::Oval)) else {
        return;
    };

    assert!(is_red(frame.pixel(32, 32)), "centre → red");
    assert!(is_red(frame.pixel(32, 14)), "top of the disc → red");
    assert!(is_red(frame.pixel(50, 32)), "right of the disc → red");
    // The square's corners fall outside the inscribed disc → erased.
    assert!(
        is_clear(frame.pixel(15, 15)),
        "top-left corner → outside the disc"
    );
    assert!(
        is_clear(frame.pixel(49, 49)),
        "bottom-right corner → outside the disc"
    );
}

/// **An arbitrary path.** A diamond, rendered as a mask by the GPU, clips the square:
/// the centre stays painted but the square's **corners**, outside the diamond, are
/// erased.
#[test]
fn path_clip_masks_to_the_shape() {
    // A diamond inscribed in the square [12, 52], its vertices at the edge midpoints.
    let diamond = Path::new()
        .move_to(Point::new(32.0, 12.0))
        .line_to(Point::new(52.0, 32.0))
        .line_to(Point::new(32.0, 52.0))
        .line_to(Point::new(12.0, 32.0))
        .close();
    let sq = Rect::new(12.0, 12.0, 40.0, 40.0);
    let Some(frame) = render(&clipped_layer(sq, ClipShape::Path(diamond))) else {
        return;
    };

    assert!(is_red(frame.pixel(32, 32)), "centre of the diamond → red");
    assert!(is_red(frame.pixel(32, 16)), "top vertex → red");
    assert!(is_red(frame.pixel(32, 48)), "bottom vertex → red");
    // The square's corners fall outside the diamond → erased by the mask.
    assert!(
        is_clear(frame.pixel(15, 15)),
        "top-left corner → outside the diamond"
    );
    assert!(
        is_clear(frame.pixel(49, 49)),
        "bottom-right corner → outside the diamond"
    );
}

/// **Zero radius means rectangle.** `RRect(0)` erases no corner and the square stays
/// solid — a guard that the rounded shape does degenerate to the rectangular clip.
#[test]
fn rrect_zero_radius_is_a_plain_rect() {
    let sq = Rect::new(12.0, 12.0, 40.0, 40.0);
    let Some(frame) = render(&clipped_layer(
        sq,
        ClipShape::RRect(BorderRadius::uniform(0.0)),
    )) else {
        return;
    };

    assert!(
        is_red(frame.pixel(14, 14)),
        "corner kept, the radius being zero"
    );
    assert!(
        is_red(frame.pixel(50, 50)),
        "corner kept, the radius being zero"
    );
    // Outside the square: nothing.
    assert!(is_clear(frame.pixel(4, 4)), "outside the box → background");
}
