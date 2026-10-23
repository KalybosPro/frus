//! Lottie animations written by hand, small enough to know what each frame must hold.

use frus_core::{
    Alignment, AlignmentGeometry, BoxFit, FillRule, PathGradient, PathVerb, Point, Primitive, Rect,
    Scene,
};
use frus_lottie::Animation;

/// A shape layer holding `shapes` (Lottie shape items, JSON), at `position` animated from
/// `from` at frame 0 to `to` at frame 60, at `opacity` (0..100), in a 100×100, 30 fps,
/// 2-second animation.
fn animation(shapes: &str, from: [f32; 2], to: [f32; 2], opacity: f32) -> String {
    format!(
        r#"{{"v":"5.7.4","fr":30,"ip":0,"op":60,"w":100,"h":100,"nm":"t","ddd":0,"assets":[],
        "layers":[{{"ddd":0,"ind":1,"ty":4,"nm":"shape","sr":1,"ip":0,"op":60,"st":0,"bm":0,
          "ks":{{"o":{{"a":0,"k":{opacity}}},"r":{{"a":0,"k":0}},
                "p":{{"a":1,"k":[{{"t":0,"s":[{},{},0],"i":{{"x":[1],"y":[1]}},"o":{{"x":[0],"y":[0]}}}},
                                {{"t":60,"s":[{},{},0]}}]}},
                "a":{{"a":0,"k":[0,0,0]}},"s":{{"a":0,"k":[100,100,100]}}}},
          "shapes":[{{"ty":"gr","nm":"g","it":[{shapes},
            {{"ty":"tr","p":{{"a":0,"k":[0,0]}},"a":{{"a":0,"k":[0,0]}},"s":{{"a":0,"k":[100,100]}},
             "r":{{"a":0,"k":0}},"o":{{"a":0,"k":100}},"sk":{{"a":0,"k":0}},"sa":{{"a":0,"k":0}}}}]}}]}}]}}"#,
        from[0], from[1], to[0], to[1]
    )
}

const SQUARE: &str = r#"{"ty":"rc","nm":"r","d":1,"p":{"a":0,"k":[0,0]},"s":{"a":0,"k":[20,20]},"r":{"a":0,"k":0}},
    {"ty":"fl","nm":"f","c":{"a":0,"k":[1,0,0,1]},"o":{"a":0,"k":100},"r":1}"#;

fn painted(animation: &Animation, progress: f32) -> Vec<Primitive> {
    let mut scene = Scene::new();
    scene.set_clip(Rect::new(0.0, 0.0, 100.0, 100.0));
    animation.paint(
        &mut scene,
        Rect::new(0.0, 0.0, 100.0, 100.0),
        BoxFit::Contain,
        AlignmentGeometry::Physical(Alignment::CENTER),
        progress,
        1.0,
    );
    scene.primitives().to_vec()
}

/// Every path in `primitives`, layers opened.
fn paths(primitives: &[Primitive]) -> Vec<Primitive> {
    let mut out = Vec::new();
    for p in primitives {
        match p {
            Primitive::Path { .. } => out.push(p.clone()),
            Primitive::Layer { primitives, .. } => out.extend(paths(primitives)),
            _ => {}
        }
    }
    out
}

/// The x extent of a path's points.
fn x_extent(p: &Primitive) -> (f32, f32) {
    let Primitive::Path { path, .. } = p else {
        panic!("a path")
    };
    let xs: Vec<f32> = path
        .verbs()
        .iter()
        .filter_map(|v| match *v {
            PathVerb::MoveTo(p) | PathVerb::LineTo(p) => Some(p.x),
            PathVerb::QuadTo { to, .. } | PathVerb::CubicTo { to, .. } => Some(to.x),
            PathVerb::Close => None,
        })
        .collect();
    (
        xs.iter().copied().fold(f32::MAX, f32::min),
        xs.iter().copied().fold(f32::MIN, f32::max),
    )
}

#[test]
fn an_animation_says_its_size_and_length() {
    let a = Animation::parse(animation(SQUARE, [20.0, 50.0], [80.0, 50.0], 100.0).as_bytes())
        .expect("parses");
    assert_eq!((a.size().width, a.size().height), (100.0, 100.0));
    assert!((a.duration() - 2.0).abs() < 1e-6);
    assert_eq!(a.frame_rate(), 30.0);
}

#[test]
fn a_square_moves_with_its_animated_position() {
    let a =
        Animation::parse(animation(SQUARE, [20.0, 50.0], [80.0, 50.0], 100.0).as_bytes()).unwrap();
    let start = paths(&painted(&a, 0.0));
    let end = paths(&painted(&a, 1.0));
    assert_eq!(start.len(), 1, "{start:?}");
    let (x0, x1) = x_extent(&start[0]);
    assert!(
        (x0 - 10.0).abs() < 0.5 && (x1 - 30.0).abs() < 0.5,
        "at the start: {x0}..{x1}"
    );
    // The last frame is frame 59 of 60: one step short of 80, linearly.
    let (x0, x1) = x_extent(&end[0]);
    assert!(
        (x0 - 69.0).abs() < 0.5 && (x1 - 89.0).abs() < 0.5,
        "at the end: {x0}..{x1}"
    );
    let Primitive::Path { fill, path, .. } = &start[0] else {
        unreachable!()
    };
    assert_eq!(*fill, Some(frus_core::Color::rgba(1.0, 0.0, 0.0, 1.0)));
    assert_eq!(path.fill_rule(), FillRule::NonZero);
}

#[test]
fn the_animation_is_fitted_into_its_box() {
    let a =
        Animation::parse(animation(SQUARE, [50.0, 50.0], [50.0, 50.0], 100.0).as_bytes()).unwrap();
    let mut scene = Scene::new();
    scene.set_clip(Rect::new(0.0, 0.0, 400.0, 400.0));
    a.paint(
        &mut scene,
        Rect::new(100.0, 0.0, 200.0, 200.0),
        BoxFit::Contain,
        AlignmentGeometry::Physical(Alignment::CENTER),
        0.0,
        1.0,
    );
    // The 20-unit square in the middle of a 100-unit animation shown 200 wide at x = 100.
    let (x0, x1) = x_extent(&paths(scene.primitives())[0]);
    assert!(
        (x0 - 180.0).abs() < 0.5 && (x1 - 220.0).abs() < 0.5,
        "{x0}..{x1}"
    );
}

#[test]
fn a_stroke_is_an_outline_around_its_line() {
    let shapes = r#"{"ty":"rc","nm":"r","d":1,"p":{"a":0,"k":[0,0]},"s":{"a":0,"k":[20,20]},"r":{"a":0,"k":0}},
        {"ty":"st","nm":"s","c":{"a":0,"k":[0,0,1,1]},"o":{"a":0,"k":100},"w":{"a":0,"k":4},"lc":2,"lj":2}"#;
    let a =
        Animation::parse(animation(shapes, [50.0, 50.0], [50.0, 50.0], 100.0).as_bytes()).unwrap();
    let drawn = paths(&painted(&a, 0.0));
    assert_eq!(drawn.len(), 1, "{drawn:?}");
    let (x0, x1) = x_extent(&drawn[0]);
    // The square's sides at 40 and 60, the stroke 2 either side of them.
    assert!(
        (x0 - 38.0).abs() < 0.5 && (x1 - 62.0).abs() < 0.5,
        "{x0}..{x1}"
    );
    let Primitive::Path { fill, path, .. } = &drawn[0] else {
        unreachable!()
    };
    assert_eq!(*fill, Some(frus_core::Color::rgba(0.0, 0.0, 1.0, 1.0)));
    assert_eq!(path.fill_rule(), FillRule::NonZero);
}

#[test]
fn a_gradient_fill_is_a_ramp_across_its_shape() {
    // A linear gradient from (-10, 0) to (10, 0) in the shape's space: black to white.
    let shapes = r#"{"ty":"rc","nm":"r","d":1,"p":{"a":0,"k":[0,0]},"s":{"a":0,"k":[20,20]},"r":{"a":0,"k":0}},
        {"ty":"gf","nm":"g","o":{"a":0,"k":100},"r":1,"t":1,"s":{"a":0,"k":[-10,0]},"e":{"a":0,"k":[10,0]},
         "g":{"p":2,"k":{"a":0,"k":[0,0,0,0,1,1,1,1]}}}"#;
    let a =
        Animation::parse(animation(shapes, [50.0, 50.0], [50.0, 50.0], 100.0).as_bytes()).unwrap();
    let drawn = paths(&painted(&a, 0.0));
    let Primitive::Path {
        gradient: Some(PathGradient::Ramp(ramp)),
        ..
    } = &drawn[0]
    else {
        panic!("a ramp: {drawn:?}")
    };
    assert_eq!(ramp.stops.len(), 2);
    // The left side of the square (x = 40) is the start, its middle halfway.
    assert!(ramp.t_at(Point::new(40.0, 50.0)).unwrap() < 0.01);
    assert!((ramp.t_at(Point::new(50.0, 50.0)).unwrap() - 0.5).abs() < 0.01);
}

#[test]
fn a_faded_layer_is_painted_faded() {
    let a =
        Animation::parse(animation(SQUARE, [50.0, 50.0], [50.0, 50.0], 50.0).as_bytes()).unwrap();
    let drawn = paths(&painted(&a, 0.0));
    let Primitive::Path {
        fill: Some(fill), ..
    } = &drawn[0]
    else {
        panic!()
    };
    // velato folds a layer's opacity into its colours.
    assert!((fill.a - 0.5).abs() < 0.01, "{fill:?}");
}

/// A 2×1 PNG: a red pixel, then a blue one.
const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAIAAAABCAYAAAD0In+KAAAADklEQVR4nGP4z8AAQv8BD/kD/YURmXYAAAAASUVORK5CYII=";

/// One image layer at (30, 40), at `opacity` (0..100), showing a 40×20 asset that the file
/// gives at `location` — its folder, then its file name or a `data:` address.
fn image_layer(folder: &str, file: &str, opacity: f32) -> String {
    format!(
        r#"{{"v":"5.7.4","fr":30,"ip":0,"op":60,"w":100,"h":100,"nm":"t","ddd":0,
        "assets":[{{"id":"image_0","w":40,"h":20,"u":"{folder}","p":"{file}","e":0}}],
        "layers":[{{"ddd":0,"ind":1,"ty":2,"nm":"picture","refId":"image_0","sr":1,"ip":0,
          "op":60,"st":0,"bm":0,
          "ks":{{"o":{{"a":0,"k":{opacity}}},"r":{{"a":0,"k":0}},"p":{{"a":0,"k":[30,40,0]}},
                "a":{{"a":0,"k":[0,0,0]}},"s":{{"a":0,"k":[100,100,100]}}}}}}]}}"#
    )
}

/// Every image in `primitives`, layers opened.
fn images(primitives: &[Primitive]) -> Vec<Primitive> {
    let mut out = Vec::new();
    for p in primitives {
        match p {
            Primitive::Image { .. } => out.push(p.clone()),
            Primitive::Layer { primitives, .. } => out.extend(images(primitives)),
            _ => {}
        }
    }
    out
}

#[test]
fn an_embedded_image_is_decoded_and_drawn_where_its_layer_puts_it() {
    let data = format!("data:image/png;base64,{PNG}");
    let a = Animation::parse(image_layer("", &data, 50.0).as_bytes()).unwrap();
    assert!(a.missing_images().is_empty());
    let drawn = images(&painted(&a, 0.0));
    let Primitive::Image {
        image, rect, tint, ..
    } = &drawn[0]
    else {
        panic!("{drawn:?}")
    };
    assert_eq!((image.width(), image.height()), (2, 1));
    let want = Rect::new(30.0, 40.0, 40.0, 20.0);
    assert!(
        (rect.x - want.x).abs() < 1e-3
            && (rect.y - want.y).abs() < 1e-3
            && (rect.width - want.width).abs() < 1e-3
            && (rect.height - want.height).abs() < 1e-3,
        "{rect:?}"
    );
    assert!((tint.a - 0.5).abs() < 0.01, "the layer's opacity: {tint:?}");
    assert!(a.unsupported().is_empty(), "{:?}", a.unsupported());
}

#[test]
fn an_image_the_file_names_is_drawn_once_the_application_gives_it() {
    let a = Animation::parse(image_layer("images/", "img_0.png", 100.0).as_bytes()).unwrap();
    assert_eq!(
        a.missing_images(),
        [("image_0".to_string(), "images/img_0.png".to_string())]
    );
    assert!(images(&painted(&a, 0.0)).is_empty());
    assert_eq!(a.unsupported(), ["image"]);

    // Pixels of another shape than the asset's: fitted whole into it, centred.
    let square = frus_core::ImageData::from_rgba(2, 2, vec![255; 16]).into_handle();
    let a = a.with_image("image_0", square);
    let drawn = images(&painted(&a, 0.0));
    let Primitive::Image { rect, .. } = &drawn[0] else {
        panic!("{drawn:?}")
    };
    // 20×20 in the middle of the 40×20 asset at (30, 40).
    assert!(
        (rect.x - 40.0).abs() < 1e-3 && (rect.width - 20.0).abs() < 1e-3,
        "{rect:?}"
    );
}

#[test]
fn a_broken_embedded_image_is_named() {
    let a =
        Animation::parse(image_layer("", "data:image/png;base64,AAAA", 100.0).as_bytes()).unwrap();
    assert_eq!(a.unsupported(), ["image"]);
    assert!(images(&painted(&a, 0.0)).is_empty());
}

#[test]
fn garbage_is_an_error() {
    assert!(Animation::parse(b"not a lottie").is_err());
}
