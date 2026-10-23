//! **A Lottie animation as painted**, read back from the GPU (milestone 691): the frame shown
//! is the moment asked for, in the colour the file gives.

use frus_core::{Alignment, AlignmentGeometry, BoxFit, Color, Rect, Scene};
use frus_lottie::Animation;
use frus_test::render_scene;

/// A red 20-unit square whose centre moves from (20, 50) to (80, 50) over 60 frames, in a
/// 100-unit animation.
const MOVING: &str = r#"{"v":"5.7.4","fr":30,"ip":0,"op":60,"w":100,"h":100,"nm":"t","ddd":0,
  "assets":[],"layers":[{"ddd":0,"ind":1,"ty":4,"nm":"s","sr":1,"ip":0,"op":60,"st":0,"bm":0,
  "ks":{"o":{"a":0,"k":100},"r":{"a":0,"k":0},
        "p":{"a":1,"k":[{"t":0,"s":[20,50,0],"i":{"x":[1],"y":[1]},"o":{"x":[0],"y":[0]}},
                        {"t":60,"s":[80,50,0]}]},
        "a":{"a":0,"k":[0,0,0]},"s":{"a":0,"k":[100,100,100]}},
  "shapes":[{"ty":"gr","nm":"g","it":[
    {"ty":"rc","nm":"r","d":1,"p":{"a":0,"k":[0,0]},"s":{"a":0,"k":[20,20]},"r":{"a":0,"k":0}},
    {"ty":"fl","nm":"f","c":{"a":0,"k":[1,0,0,1]},"o":{"a":0,"k":100},"r":1},
    {"ty":"tr","p":{"a":0,"k":[0,0]},"a":{"a":0,"k":[0,0]},"s":{"a":0,"k":[100,100]},
     "r":{"a":0,"k":0},"o":{"a":0,"k":100},"sk":{"a":0,"k":0},"sa":{"a":0,"k":0}}]}]}]}"#;

const SIZE: u32 = 200;

fn frame(progress: f32) -> Option<frus_test::Snapshot> {
    let animation = Animation::parse(MOVING.as_bytes()).unwrap();
    let mut scene = Scene::new();
    scene.set_clip(Rect::new(0.0, 0.0, SIZE as f32, SIZE as f32));
    animation.paint(
        &mut scene,
        Rect::new(0.0, 0.0, SIZE as f32, SIZE as f32),
        BoxFit::Contain,
        AlignmentGeometry::Physical(Alignment::CENTER),
        progress,
        1.0,
    );
    render_scene(&scene, SIZE, SIZE, Color::WHITE)
}

fn red(pixel: [u8; 4]) -> bool {
    pixel[0] > 230 && pixel[1] < 25 && pixel[2] < 25
}

#[test]
fn the_square_is_where_the_moment_puts_it() {
    let Some(start) = frame(0.0) else {
        return;
    };
    // Twice the animation's units: the square spans 20..60 on the first frame.
    assert!(
        red(start.pixel(40, 100)),
        "at the start: {:?}",
        start.pixel(40, 100)
    );
    assert!(!red(start.pixel(150, 100)), "and not at the end's place");
    let end = frame(1.0).expect("rendered once already");
    // The last frame, 59 of 60: the square spans 138..178.
    assert!(
        red(end.pixel(158, 100)),
        "at the end: {:?}",
        end.pixel(158, 100)
    );
    assert!(!red(end.pixel(40, 100)), "and gone from the start");
}

/// A 2×1 PNG — a red pixel, then a blue one — shown 40×20 at (30, 40) and turned a quarter
/// about that corner, over a 100-unit animation.
const TURNED_IMAGE: &str = r#"{"v":"5.7.4","fr":30,"ip":0,"op":60,"w":100,"h":100,"nm":"t",
  "ddd":0,"assets":[{"id":"image_0","w":40,"h":20,"u":"","e":1,
    "p":"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAIAAAABCAYAAAD0In+KAAAADklEQVR4nGP4z8AAQv8BD/kD/YURmXYAAAAASUVORK5CYII="}],
  "layers":[{"ddd":0,"ind":1,"ty":2,"nm":"picture","refId":"image_0","sr":1,"ip":0,"op":60,
    "st":0,"bm":0,"ks":{"o":{"a":0,"k":100},"r":{"a":0,"k":90},"p":{"a":0,"k":[50,30,0]},
    "a":{"a":0,"k":[0,0,0]},"s":{"a":0,"k":[100,100,100]}}}]}"#;

#[test]
fn an_image_layer_is_drawn_turned_with_its_layer() {
    let animation = Animation::parse(TURNED_IMAGE.as_bytes()).unwrap();
    let mut scene = Scene::new();
    scene.set_clip(Rect::new(0.0, 0.0, SIZE as f32, SIZE as f32));
    animation.paint(
        &mut scene,
        Rect::new(0.0, 0.0, SIZE as f32, SIZE as f32),
        BoxFit::Contain,
        AlignmentGeometry::Physical(Alignment::CENTER),
        0.0,
        1.0,
    );
    let Some(shot) = render_scene(&scene, SIZE, SIZE, Color::WHITE) else {
        return;
    };
    assert!(
        animation.unsupported().is_empty(),
        "{:?}",
        animation.unsupported()
    );
    // Turned a quarter about (50, 30): it spans x 30..50, y 30..70, its left at the top.
    // Twice the animation's units on screen.
    let blue = |p: [u8; 4]| p[2] > 200 && p[0] < 40 && p[1] < 40;
    assert!(red(shot.pixel(80, 70)), "left: {:?}", shot.pixel(80, 70));
    assert!(
        blue(shot.pixel(80, 130)),
        "right: {:?}",
        shot.pixel(80, 130)
    );
    let white = |p: [u8; 4]| p[0] > 245 && p[1] > 245 && p[2] > 245;
    assert!(
        white(shot.pixel(50, 100)) && white(shot.pixel(110, 100)),
        "nothing beside it"
    );
}
