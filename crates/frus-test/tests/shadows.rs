//! Shadows, **rendered** (milestone 606).
//!
//! A shadow's rectangle already reaches its blur beyond the shape it is the shadow of, and
//! the renderer draws nothing outside a rectangle. So the whole fade has to happen inside
//! it: at full strength where the shape's edge less the blur is, and at nothing where the
//! rectangle ends. The renderer used to fade across the rectangle's edge instead, so the
//! outer half was never drawn and every shadow stopped short, at half its strength, in a
//! hard grey line: the grey box around the text selection bar that a phone showed.

use frus_core::{paint_elevation, BorderRadius, BoxShadow, Color, Rect, Scene};
use frus_test::render_scene;

/// The darkest a channel of a white page gets, going out along a row from `from` to `to`.
fn darkest(shot: &frus_test::Snapshot, y: u32, from: u32, to: u32) -> Vec<u8> {
    (from..to).map(|x| shot.pixel(x, y)[0]).collect()
}

/// **A shadow fades out to nothing at the edge of what is drawn**, with no step there: going
/// out from the box, the page gets lighter and lighter, and the last pixel the shadow's
/// rectangle covers is the page's own white or nearly.
#[test]
fn a_shadow_fades_to_nothing_with_no_step_at_its_edge() {
    let rect = Rect::new(60.0, 40.0, 60.0, 40.0);
    let mut scene = Scene::new();
    scene.fill_rect(Rect::new(0.0, 0.0, 200.0, 140.0), Color::WHITE);
    let shadow = BoxShadow::new(0.0, 0.0, 12.0, Color::rgba(0.0, 0.0, 0.0, 0.8));
    scene.shadow(
        shadow.bounds(rect),
        shadow.color,
        BorderRadius::ZERO.inflate(12.0),
        12.0,
    );
    let Some(shot) = render_scene(&scene, 200, 140, Color::WHITE) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    // From the box's right edge out past where the shadow's rectangle ends (at 132).
    let row = darkest(&shot, 60, 120, 140);
    for pair in row.windows(2) {
        assert!(pair[1] >= pair[0], "lighter and lighter going out: {row:?}");
    }
    let last_inside = row[(131 - 120) as usize];
    assert!(last_inside >= 250, "nothing left at the edge: {row:?}");
    // And at the box's own edge, the shadow is at about half its strength: 0.8 of black
    // halved is 0.4 over white, blended in linear light, so 0.6 of white there — about 203
    // once written back in sRGB.
    let at_edge = row[0];
    assert!(
        (195..=215).contains(&at_edge),
        "half strength at the box's edge: {row:?}"
    );
}

/// **A surface one high casts a faint, tight shadow**: under a 60×40 box on white, the
/// darkest pixel just below it is light grey, and ten pixels further out there is nothing.
#[test]
fn a_surface_one_high_casts_a_faint_tight_shadow() {
    let rect = Rect::new(60.0, 40.0, 60.0, 40.0);
    let mut scene = Scene::new();
    scene.fill_rect(Rect::new(0.0, 0.0, 200.0, 140.0), Color::WHITE);
    paint_elevation(
        &mut scene,
        rect,
        BorderRadius::uniform(8.0),
        1.0,
        Color::BLACK,
    );
    scene.fill_rect(rect, Color::WHITE);
    let Some(shot) = render_scene(&scene, 200, 140, Color::WHITE) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    let under = shot.pixel(90, 81)[0];
    assert!(
        (150..250).contains(&under),
        "a faint shadow just under: {under}"
    );
    let further = shot.pixel(90, 92)[0];
    assert!(further >= 252, "and nothing ten pixels on: {further}");
}
