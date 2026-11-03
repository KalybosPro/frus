// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! **An image drawn much smaller than its pixels averages them** (milestone 743).
//!
//! At one eighth of its size, each screen pixel covers sixty-four of a picture's. A
//! bilinear sample reads four of them, so a fine pattern comes out as whichever texels the
//! samples happen to land on: a checkerboard turns solid black or solid white, thin lines
//! vanish or fill the picture, and the result changes as the scale moves. The mip chain
//! averages them first, so a fine pattern shows as the tone it has from afar.
//!
//! Each picture here is drawn a sixteenth of a pixel off the pixel grid, which puts every
//! bilinear sample of the first level on a texel centre: without a chain, the sample is
//! exactly one texel and the aliasing is total, so a test that only accepts the average
//! cannot pass by luck.
//!
//! The average is in **linear light**: half black and half white is linear 0.5, sRGB 188,
//! not 128. An average taken on the sRGB bytes would be the colour-space slip this project
//! keeps catching.

use frus_core::{Color, FilterQuality, ImageData, ImageHandle, Rect, Scene};
use frus_test::{render_scene, Snapshot};

const SIDE: u32 = 256;
/// One eighth of the picture.
const SHOWN: f32 = 32.0;
/// A sixteenth of a pixel off the grid: see the module notes.
const AT: f32 = 8.0 + 1.0 / 16.0;
const CANVAS: u32 = 48;

/// sRGB of linear 0.5: what half black and half white looks like from afar.
const GREY: f32 = 187.5;

fn picture(texel: impl Fn(u32, u32) -> [u8; 4]) -> ImageHandle {
    let mut rgba = Vec::with_capacity((SIDE * SIDE * 4) as usize);
    for y in 0..SIDE {
        for x in 0..SIDE {
            rgba.extend_from_slice(&texel(x, y));
        }
    }
    ImageData::from_rgba(SIDE, SIDE, rgba).into_handle()
}

/// Black and white texels, one pixel each.
fn checkerboard() -> ImageHandle {
    picture(|x, y| {
        if (x + y) % 2 == 0 {
            [255, 255, 255, 255]
        } else {
            [0, 0, 0, 255]
        }
    })
}

fn render(scene: &Scene) -> Option<Snapshot> {
    let frame = render_scene(scene, CANVAS, CANVAS, Color::BLACK);
    if frame.is_none() {
        eprintln!("no GPU adapter available: test skipped");
    }
    frame
}

/// Draws `image` at one eighth, off the grid, at `quality`.
fn shrunk(image: &ImageHandle, quality: FilterQuality) -> Scene {
    let mut scene = Scene::new();
    let whole = Rect::new(0.0, 0.0, 1.0, 1.0);
    let rect = Rect::new(AT, AT, SHOWN, SHOWN);
    scene.draw_image_with(image, rect, whole, Color::WHITE, false, quality);
    scene
}

/// The pixels well inside the drawn picture, clear of its edges.
fn inside(frame: &Snapshot) -> Vec<[u8; 4]> {
    let mut pixels = Vec::new();
    for y in 10..38 {
        for x in 10..38 {
            pixels.push(frame.pixel(x, y));
        }
    }
    pixels
}

/// The mean and the spread (largest less smallest) of one channel.
fn stats(pixels: &[[u8; 4]], channel: usize) -> (f32, u8) {
    let values: Vec<u8> = pixels.iter().map(|p| p[channel]).collect();
    let mean = values.iter().map(|&v| f32::from(v)).sum::<f32>() / values.len() as f32;
    let spread = values.iter().max().unwrap() - values.iter().min().unwrap();
    (mean, spread)
}

#[test]
fn a_fine_checkerboard_at_one_eighth_is_an_even_grey() {
    let image = checkerboard();
    for quality in [FilterQuality::Medium, FilterQuality::High] {
        let Some(frame) = render(&shrunk(&image, quality)) else {
            return;
        };
        let pixels = inside(&frame);
        for channel in 0..3 {
            let (mean, spread) = stats(&pixels, channel);
            assert!(
                (mean - GREY).abs() < 6.0,
                "{quality:?}: channel {channel} averages {mean}, not the grey {GREY}"
            );
            assert!(
                spread <= 6,
                "{quality:?}: channel {channel} varies by {spread}: the pattern shows through"
            );
        }
    }
}

/// The default a scene draws with is the one that averages.
#[test]
fn a_plain_draw_averages_too() {
    let image = checkerboard();
    let mut scene = Scene::new();
    let whole = Rect::new(0.0, 0.0, 1.0, 1.0);
    scene.draw_image(&image, Rect::new(AT, AT, SHOWN, SHOWN), whole, Color::WHITE);
    let Some(frame) = render(&scene) else {
        return;
    };
    let (mean, spread) = stats(&inside(&frame), 1);
    assert!((mean - GREY).abs() < 6.0, "averages {mean}");
    assert!(spread <= 6, "varies by {spread}");
}

/// What the chain is for: `Low` and `None` read the first level only, and the
/// checkerboard aliases to one of its two colours. Kept as a test so the samplers stay
/// honest about the quality asked for.
#[test]
fn without_the_chain_the_checkerboard_aliases() {
    let image = checkerboard();
    for quality in [FilterQuality::Low, FilterQuality::None] {
        let Some(frame) = render(&shrunk(&image, quality)) else {
            return;
        };
        for pixel in inside(&frame) {
            assert!(
                pixel[1] < 24 || pixel[1] > 231,
                "{quality:?}: {pixel:?} is a blend; this quality must not average"
            );
        }
    }
}

/// One-pixel white lines, one column in eight, average to an eighth of white in linear
/// light: neither vanish nor fill the picture.
#[test]
fn thin_lines_average_to_their_coverage() {
    // Column 3 of every eight: the column every off-grid first-level sample lands on, so
    // a draw without the chain shows nothing but white.
    let lines = picture(|x, _| {
        if x % 8 == 3 {
            [255, 255, 255, 255]
        } else {
            [0, 0, 0, 255]
        }
    });
    let Some(frame) = render(&shrunk(&lines, FilterQuality::Medium)) else {
        return;
    };
    // sRGB of linear 1/8.
    let expected = 255.0 * (1.055 * (0.125f32).powf(1.0 / 2.4) - 0.055);
    let (mean, spread) = stats(&inside(&frame), 0);
    assert!(
        (mean - expected).abs() < 6.0,
        "lines average to {mean}, not {expected}"
    );
    assert!(spread <= 6, "varies by {spread}");

    let Some(aliased) = render(&shrunk(&lines, FilterQuality::Low)) else {
        return;
    };
    let (mean, _) = stats(&inside(&aliased), 0);
    assert!(
        mean > 231.0,
        "without the chain the lines fill the picture: {mean}"
    );
}

/// The chain averages colours **weighted by their alpha**: the colour hidden in a
/// transparent texel does not leak into its opaque neighbours.
#[test]
fn transparent_texels_do_not_tint_the_average() {
    // Opaque red, and transparent texels that carry green.
    let image = picture(|x, y| {
        if (x + y) % 2 == 0 {
            [255, 0, 0, 255]
        } else {
            [0, 255, 0, 0]
        }
    });
    let Some(frame) = render(&shrunk(&image, FilterQuality::Medium)) else {
        return;
    };
    let pixels = inside(&frame);
    let (red, _) = stats(&pixels, 0);
    let (green, _) = stats(&pixels, 1);
    // Half-covered red over black: linear 0.5 of red.
    assert!((red - GREY).abs() < 6.0, "red {red}");
    assert!(
        green < 4.0,
        "green {green} leaked from the transparent texels"
    );
}

/// A picture at its own size reads its pixels exactly, chain or none, even beside a
/// shrunk draw of the same picture that gives it a chain mid-frame.
#[test]
fn a_picture_at_its_own_size_is_untouched() {
    let image = picture(|x, y| {
        [
            (x % 256) as u8,
            (y % 256) as u8,
            ((x * 7 + y) % 256) as u8,
            255,
        ]
    });
    let whole = Rect::new(0.0, 0.0, 1.0, 1.0);
    for first in [
        FilterQuality::Low,
        FilterQuality::Medium,
        FilterQuality::None,
    ] {
        let mut scene = Scene::new();
        // Its own size first, then shrunk: the second draw remakes the texture.
        scene.draw_image_with(
            &image,
            Rect::new(0.0, 0.0, SIDE as f32, SIDE as f32),
            whole,
            Color::WHITE,
            false,
            first,
        );
        scene.draw_image_with(
            &image,
            Rect::new(SIDE as f32, 0.0, SHOWN, SHOWN),
            whole,
            Color::WHITE,
            false,
            FilterQuality::Medium,
        );
        let Some(frame) = render_scene(&scene, SIDE + 40, SIDE, Color::BLACK) else {
            eprintln!("no GPU adapter available: test skipped");
            return;
        };
        for (x, y) in [(0, 0), (5, 9), (100, 37), (255, 255), (128, 200)] {
            let px = frame.pixel(x, y);
            let want = [x as u8, y as u8, ((x * 7 + y) % 256) as u8];
            for c in 0..3 {
                assert!(
                    px[c].abs_diff(want[c]) <= 1,
                    "{first:?}: pixel ({x}, {y}) is {px:?}, not {want:?}"
                );
            }
        }
    }
}
