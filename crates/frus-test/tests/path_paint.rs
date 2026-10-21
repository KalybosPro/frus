//! **A path's gradient with stops, and its fill rule, as painted** (milestone 689).
//!
//! `GradientRamp::t_at` is the reckoning, tested on its own in `frus-core`; `path.wgsl`
//! repeats it per pixel. These render a ramp-filled shape and read pixels back against the
//! CPU's answer, so the two cannot drift apart unnoticed — a swapped focal point, a tile
//! mode off by one, a transform applied the wrong way round all give a plausible gradient
//! and the wrong one.

use frus_core::{
    Affine, Color, FillRule, GradientRamp, Path, Point, RampShape, Rect, Scene, TileMode,
};
use frus_test::render_scene;

const SIZE: u32 = 128;

/// Three stops, mid-range and different on every channel, so a slip moves a pixel.
fn stops() -> Vec<(f32, Color)> {
    vec![
        (0.0, Color::rgb(0.9, 0.2, 0.3)),
        (0.4, Color::rgb(0.2, 0.7, 0.4)),
        (1.0, Color::rgb(0.3, 0.3, 0.8)),
    ]
}

/// The whole canvas filled with `ramp`, every probe compared with the CPU's colour at the
/// pixel's centre. Returns quietly where there is no GPU, as every test here does.
fn paints_as_reckoned(ramp: GradientRamp, probes: &[(u32, u32)]) {
    let mut scene = Scene::new();
    let canvas = Path::rect(Rect::new(0.0, 0.0, SIZE as f32, SIZE as f32));
    scene.fill_path_ramp(&canvas, ramp.clone(), 1.0);
    let Some(shot) = render_scene(&scene, SIZE, SIZE, Color::BLACK) else {
        return;
    };
    for &(x, y) in probes {
        let want = ramp.color_at(Point::new(x as f32 + 0.5, y as f32 + 0.5));
        let want = [want.r, want.g, want.b].map(|v| (v * 255.0).round() as i32);
        let got = shot.pixel(x, y);
        let off = (0..3)
            .map(|c| (i32::from(got[c]) - want[c]).abs())
            .max()
            .unwrap();
        // The ramp is 256 texels, filtered: a step of a level or two either way.
        assert!(
            off <= 3,
            "at ({x}, {y}) painted {got:?}, reckoned {want:?}, for {:?}",
            ramp.shape
        );
    }
}

fn grid() -> Vec<(u32, u32)> {
    (0..8)
        .flat_map(|i| (0..8).map(move |j| (i * 16 + 5, j * 16 + 9)))
        .collect()
}

#[test]
fn a_linear_ramp_paints_its_stops() {
    paints_as_reckoned(
        GradientRamp {
            shape: RampShape::Linear {
                from: Point::new(16.0, 0.0),
                to: Point::new(112.0, 0.0),
            },
            stops: stops(),
            tile: TileMode::Clamp,
            transform: Affine::default(),
        },
        &grid(),
    );
}

#[test]
fn a_radial_ramp_with_a_focal_point_paints_as_reckoned() {
    paints_as_reckoned(
        GradientRamp {
            shape: RampShape::Radial {
                center: Point::new(64.0, 64.0),
                radius: 56.0,
                focal: Point::new(84.0, 50.0),
                focal_radius: 4.0,
            },
            stops: stops(),
            tile: TileMode::Clamp,
            transform: Affine::default(),
        },
        &grid(),
    );
}

#[test]
fn a_turned_and_repeated_ramp_paints_as_reckoned() {
    paints_as_reckoned(
        GradientRamp {
            shape: RampShape::Linear {
                from: Point::new(0.0, 0.0),
                to: Point::new(40.0, 0.0),
            },
            stops: stops(),
            tile: TileMode::Repeated,
            transform: Affine::rotation(0.6)
                .then(Affine::scale(1.0, 0.5))
                .then(Affine::translation(20.0, 10.0)),
        },
        &grid(),
    );
}

/// Two squares, one inside the other, wound the same way: even-odd makes the inner one a
/// hole, non-zero fills it.
#[test]
fn the_fill_rule_decides_whether_a_nested_contour_is_a_hole() {
    let square = |r: Rect| {
        Path::new()
            .move_to(Point::new(r.x, r.y))
            .line_to(Point::new(r.x + r.width, r.y))
            .line_to(Point::new(r.x + r.width, r.y + r.height))
            .line_to(Point::new(r.x, r.y + r.height))
            .close()
    };
    let nested =
        square(Rect::new(8.0, 8.0, 112.0, 112.0)).append(square(Rect::new(40.0, 40.0, 48.0, 48.0)));
    let middle = |rule: FillRule| {
        let mut scene = Scene::new();
        scene.fill_path(&nested.clone().with_fill_rule(rule), Color::WHITE);
        render_scene(&scene, SIZE, SIZE, Color::BLACK)
            .map(|shot| (shot.pixel(64, 64), shot.pixel(20, 64)))
    };
    let Some((hole, ring)) = middle(FillRule::EvenOdd) else {
        return;
    };
    assert_eq!(hole[0], 0, "even-odd: the inner square is a hole");
    assert_eq!(ring[0], 255, "and the ring between them is filled");
    let (filled, _) = middle(FillRule::NonZero).expect("rendered once already");
    assert_eq!(filled[0], 255, "non-zero: the inner square is filled");
}
