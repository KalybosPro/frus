// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! Shadows, **rendered** (milestones 606 and 746).
//!
//! A shadow's rectangle reaches its blur beyond the shape it is the shadow of. The renderer
//! used to fade across the rectangle's edge, so the outer half was never drawn and every
//! shadow stopped short, at half its strength, in a hard grey line: the grey box around the
//! text selection bar that a phone showed (606). It then faded inside the rectangle, ending
//! at its edge, which still cut the widest shadows short of where a blur ends (746): now a
//! shadow is its shape blurred by a gaussian, drawn as far out as the gaussian reaches.

use frus_core::{paint_elevation, BorderRadius, BoxShadow, Color, Rect, Scene};
use frus_test::render_scene;

/// The darkest a channel of a white page gets, going out along a row from `from` to `to`.
fn darkest(shot: &frus_test::Snapshot, y: u32, from: u32, to: u32) -> Vec<u8> {
    (from..to).map(|x| shot.pixel(x, y)[0]).collect()
}

/// **A shadow fades out to nothing, with no step anywhere**: going out from the box, the
/// page gets lighter and lighter, past the shadow's rectangle (a blur's gaussian reaches
/// further than the blur), until it is the page's own white three deviations out.
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
    // From the box's right edge out past where the shadow's rectangle ends (at 132), and
    // past where its gaussian (deviation 7.4) does, about 22 pixels out.
    let row = darkest(&shot, 60, 120, 150);
    for pair in row.windows(2) {
        assert!(pair[1] >= pair[0], "lighter and lighter going out: {row:?}");
    }
    let last_inside = row[(131 - 120) as usize];
    assert!(
        (240..255).contains(&last_inside),
        "faint, and still fading, at the rectangle's edge: {row:?}"
    );
    assert_eq!(row[29], 255, "nothing left three deviations out: {row:?}");
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

/// **A surface twenty-four high casts a shadow that fades into the page with no rim**
/// (milestone 746). A shadow is its shape blurred by a gaussian, whose deviation is a
/// little over half the blur, and a gaussian goes on three deviations out. The renderer
/// used to stop each shadow at one blur beyond its shape, fading it there along a curve of
/// its own: the widest shadows, a surface's highest, ended on a rim around the surface, at
/// a distance where the real fade still had a third of its way to go.
///
/// The shadow is drawn in white on a black page, where blending in linear light makes the
/// faintest of tails visible: a gaussian's last tenth of a percent shows; nothing past
/// the end of what is drawn does.
#[test]
fn a_surface_twenty_four_high_fades_into_the_page_with_no_rim() {
    let rect = Rect::new(100.0, 100.0, 96.0, 64.0);
    let mut scene = Scene::new();
    paint_elevation(
        &mut scene,
        rect,
        BorderRadius::uniform(12.0),
        24.0,
        Color::WHITE,
    );
    let Some(shot) = render_scene(&scene, 400, 300, Color::BLACK) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    // Out from the surface's right edge, through its middle, to the page's.
    let row: Vec<u8> = (196..400).map(|x| shot.pixel(x, 132)[0]).collect();
    for pair in row.windows(2) {
        assert!(pair[1] <= pair[0], "darker and darker going out: {row:?}");
    }
    // The widest of the three shadows (blur 46, spread 8) used to end 54 pixels out. Its
    // gaussian, of deviation 27, is still there 66 pixels out, and gone by 100.
    assert!(row[66] > 0, "the fade goes on past one blur: {row:?}");
    assert_eq!(row[100], 0, "and ends: {row:?}");
    // No step anywhere in the fade: neighbours differ by a few levels at most, and the
    // last lit pixel is barely lit.
    let steepest = row
        .windows(2)
        .map(|pair| pair[0] - pair[1])
        .max()
        .unwrap_or(0);
    assert!(steepest <= 4, "no step in the fade ({steepest}): {row:?}");
    let last_lit = row.iter().rposition(|&v| v > 0).expect("a shadow");
    assert!(row[last_lit] <= 2, "it fades out, not off: {row:?}");
}
