//! Where a **layer** sits in the pile.
//!
//! A layer — a group opacity, a fade, a rotation — is rendered flat into its own
//! texture and then composited. That is what lets one pass transform or fade a whole
//! subtree. It also meant, until this was fixed, that *every* layer was composited after
//! *all* of the ordinary content: a scene that painted a group and then covered it had
//! the group reappear on top.
//!
//! Found on a device (milestone 349): the demo's translucent square, which belongs to
//! the home screen, was painted over the Kanban board that had covered it — and thin
//! slivers of a swipe background sat at the left edge of every screen.

use frus_core::{Color, Point, Rect, Scene};
use frus_test::render_scene;

const RED: Color = Color::rgb(1.0, 0.0, 0.0);
const GREEN: Color = Color::rgb(0.0, 1.0, 0.0);
const BLUE: Color = Color::rgb(0.0, 0.0, 1.0);

/// A group painted first and covered afterwards **stays** covered.
#[test]
fn a_layer_is_covered_by_what_is_painted_after_it() {
    let mut scene = Scene::new();
    scene.set_clip(Rect::new(0.0, 0.0, 64.0, 64.0));
    // A group, then an opaque rectangle over the whole of it.
    scene.layer(1.0, |inner| {
        inner.fill_rect(Rect::new(8.0, 8.0, 32.0, 32.0), GREEN);
    });
    scene.fill_rect(Rect::new(0.0, 0.0, 64.0, 64.0), BLUE);

    let Some(shot) = render_scene(&scene, 64, 64, RED) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    let px = shot.pixel(20, 20);
    assert!(
        px[2] > 200 && px[1] < 80,
        "the cover is on top, not the group: {px:?}"
    );
}

/// And a group painted **after** something still covers it — the half that already
/// worked, pinned so that fixing the other half does not trade one for the other.
#[test]
fn a_layer_covers_what_is_painted_before_it() {
    let mut scene = Scene::new();
    scene.set_clip(Rect::new(0.0, 0.0, 64.0, 64.0));
    scene.fill_rect(Rect::new(0.0, 0.0, 64.0, 64.0), BLUE);
    scene.layer(1.0, |inner| {
        inner.fill_rect(Rect::new(8.0, 8.0, 32.0, 32.0), GREEN);
    });

    let Some(shot) = render_scene(&scene, 64, 64, RED) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    let px = shot.pixel(20, 20);
    assert!(px[1] > 200 && px[2] < 80, "the group is on top: {px:?}");
}

/// Two groups keep their own order, and content between them is sandwiched: the second
/// group covers the middle rectangle, and the middle rectangle covers the first group.
#[test]
fn groups_and_content_interleave_in_scene_order() {
    let mut scene = Scene::new();
    scene.set_clip(Rect::new(0.0, 0.0, 64.0, 64.0));
    scene.layer(1.0, |inner| {
        inner.fill_rect(Rect::new(0.0, 0.0, 64.0, 64.0), GREEN);
    });
    scene.fill_rect(Rect::new(0.0, 0.0, 64.0, 32.0), BLUE);
    scene.layer(1.0, |inner| {
        inner.fill_rect(Rect::new(0.0, 0.0, 64.0, 16.0), RED);
    });

    let Some(shot) = render_scene(&scene, 64, 64, Color::rgb(0.0, 0.0, 0.0)) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    // Top band: the second group, over the blue.
    let top = shot.pixel(32, 8);
    assert!(top[0] > 200 && top[2] < 80, "the last group wins: {top:?}");
    // Middle: the blue, over the first group.
    let middle = shot.pixel(32, 24);
    assert!(
        middle[2] > 200 && middle[1] < 80,
        "content covers the group before it: {middle:?}"
    );
    // Bottom: the first group, uncovered.
    let bottom = shot.pixel(32, 48);
    assert!(
        bottom[1] > 200 && bottom[2] < 80,
        "the first group, where nothing covered it: {bottom:?}"
    );
}

/// The same question for a **masked** group, which is the shape a fade takes: it is a
/// layer too, and a fade that has been covered must stay covered.
#[test]
fn a_faded_group_is_covered_too() {
    let mut scene = Scene::new();
    scene.set_clip(Rect::new(0.0, 0.0, 64.0, 64.0));
    let fade = frus_core::ShaderMask::new(frus_core::MaskShader::Linear {
        from: Point::new(0.0, 0.0),
        to: Point::new(64.0, 0.0),
        from_color: Color::WHITE,
        to_color: Color::rgba(1.0, 1.0, 1.0, 0.0),
    });
    scene.masked(fade, |inner| {
        inner.fill_rect(Rect::new(0.0, 0.0, 64.0, 64.0), GREEN);
    });
    scene.fill_rect(Rect::new(0.0, 0.0, 64.0, 64.0), BLUE);

    let Some(shot) = render_scene(&scene, 64, 64, RED) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    let px = shot.pixel(8, 32);
    assert!(
        px[2] > 200 && px[1] < 80,
        "the cover is on top of the fade: {px:?}"
    );
}
