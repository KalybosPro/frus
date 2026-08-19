//! **Pixel-level** tests of the transform pipeline: a layer
//! ([`frus_core::Primitive::Layer`]) carrying an affine matrix is composited by the
//! GPU, and we check that the pixels land **where the matrix says they should**. This
//! is the piece `frus-widgets`'s tests cannot cover: those prove the *right matrix* is
//! emitted, these prove the compositing shader (`composite.wgsl`) **renders it
//! correctly**, sampling at `M⁻¹`.
//!
//! With no GPU adapter `render_scene` returns `None` and the test skips itself.
//!
//! Convention: origin at the top left, y pointing down, rotation **clockwise**.

use std::f32::consts::FRAC_PI_2;

use frus_core::{Affine, Color, LayerFilter, LayerTransform, Point, Primitive, Rect, Scene};
use frus_test::{render_scene, Snapshot};

/// A scene: a solid `inner` rectangle in `color`, wrapped in a layer transformed by
/// `m` — rendered flat, then composited transformed.
fn transformed_layer(inner: Rect, color: Color, m: LayerTransform) -> Scene {
    let mut content = Scene::new();
    content.fill_rect(inner, color);
    let primitives = content.split_off(0);
    let mut scene = Scene::new();
    scene.push_primitive(Primitive::Layer {
        primitives,
        opacity: 1.0,
        clip: Rect::UNBOUNDED,
        clip_shape: frus_core::ClipShape::Rect,
        transform: Some(m),
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

/// **Rotation.** A **horizontal** bar turned +90° about the centre becomes
/// **vertical**: its pixels are where the vertical bar would be, and no longer where
/// the horizontal one was.
#[test]
fn rotation_turns_a_horizontal_bar_vertical() {
    let red = Color::rgb(1.0, 0.0, 0.0);
    // A horizontal bar: x ∈ [8,56], y ∈ [28,36].
    let bar = Rect::new(8.0, 28.0, 48.0, 8.0);
    let m = LayerTransform::rotation(FRAC_PI_2, Point::new(32.0, 32.0));
    let Some(frame) = render(&transformed_layer(bar, red, m)) else {
        return;
    };

    // After +90° the bar is vertical: x ∈ [28,36], y ∈ [8,56].
    assert!(is_red(frame.pixel(32, 12)), "top of the vertical bar → red");
    assert!(
        is_red(frame.pixel(32, 52)),
        "bottom of the vertical bar → red"
    );
    assert!(is_red(frame.pixel(32, 32)), "centre → red");
    // Where the horizontal bar used to be, but the vertical one is not → background.
    assert!(
        is_clear(frame.pixel(12, 32)),
        "the old left edge → background, it turned"
    );
    assert!(
        is_clear(frame.pixel(52, 32)),
        "the old right edge → background, it turned"
    );
}

/// **Uniform scale.** A small square scaled ×2 about the centre covers four times the
/// area: a point outside the original square but inside its image is painted.
#[test]
fn uniform_scale_enlarges_about_center() {
    let red = Color::rgb(1.0, 0.0, 0.0);
    // A centred 16×16 square: x,y ∈ [24,40].
    let sq = Rect::new(24.0, 24.0, 16.0, 16.0);
    let m = LayerTransform::new(Affine::scale(2.0, 2.0).about(Point::new(32.0, 32.0)));
    let Some(frame) = render(&transformed_layer(sq, red, m)) else {
        return;
    };

    // The ×2 image: x,y ∈ [16,48].
    assert!(
        is_red(frame.pixel(20, 20)),
        "inside the enlarged image, outside the original square → red"
    );
    assert!(is_red(frame.pixel(32, 32)), "centre → red");
    assert!(
        is_clear(frame.pixel(6, 6)),
        "outside the image → background"
    );
}

/// **Non-uniform scale.** `scale(3, 1)` widens the square in x without touching y: a
/// point widened horizontally is painted, one displaced in y is not.
#[test]
fn non_uniform_scale_widens_x_only() {
    let red = Color::rgb(1.0, 0.0, 0.0);
    let sq = Rect::new(24.0, 24.0, 16.0, 16.0); // x,y ∈ [24,40]
    let m = LayerTransform::new(Affine::scale(3.0, 1.0).about(Point::new(32.0, 32.0)));
    let Some(frame) = render(&transformed_layer(sq, red, m)) else {
        return;
    };

    // The image: x ∈ [8,56] (×3), y ∈ [24,40], unchanged.
    assert!(is_red(frame.pixel(12, 32)), "widened in x → red");
    assert!(
        is_clear(frame.pixel(32, 12)),
        "y is not scaled → background above"
    );
    assert!(
        is_clear(frame.pixel(4, 32)),
        "beyond the widening → background"
    );
}

/// **Composition.** `scale ×2` **then** a +90° rotation, in a single matrix: the
/// square, both enlarged *and* turned, covers the expected image — `M⁻¹` sampling does
/// compose.
#[test]
fn scale_then_rotate_composes() {
    let red = Color::rgb(1.0, 0.0, 0.0);
    // A centred 16×8 square, wide and short: x ∈ [24,40], y ∈ [28,36].
    let sq = Rect::new(24.0, 28.0, 16.0, 8.0);
    let pivot = Point::new(32.0, 32.0);
    // Scale ×2 (→ 32×16) then rotate +90° (→ 16 wide by 32 tall).
    let m = LayerTransform::new(
        Affine::scale(2.0, 2.0)
            .about(pivot)
            .then(Affine::rotation(FRAC_PI_2).about(pivot)),
    );
    let Some(frame) = render(&transformed_layer(sq, red, m)) else {
        return;
    };

    // After ×2: x ∈ [16,48], y ∈ [24,40]. After +90° about the centre:
    // x ∈ [24,40], y ∈ [16,48] — tall and narrow.
    assert!(
        is_red(frame.pixel(32, 20)),
        "top of the composed image → red"
    );
    assert!(
        is_red(frame.pixel(32, 44)),
        "bottom of the composed image → red"
    );
    assert!(
        is_clear(frame.pixel(12, 32)),
        "outside the narrow image → background"
    );
    assert!(
        is_clear(frame.pixel(52, 32)),
        "outside the narrow image → background"
    );
}
