//! `AnimatedSwitcher` (milestone 502), **rendered**, halfway through a switch.
//!
//! The unit tests read the scene. These read the pixels, because the claim a switcher
//! makes is about the screen: for the length of a switch **both** children are on it, the
//! old one on its way out under the new one on its way in — which is the whole difference
//! from a jump, and the one thing a picture of either end cannot show.

use frus_core::Color;
use frus_test::Stage;
use frus_widgets::{
    text, AnimatedSwitcher, Container, FadeTransition, Flex, ScaleTransition, Theme,
};

fn golden(name: &str) -> String {
    format!("{}/tests/goldens/{name}.png", env!("CARGO_MANIFEST_DIR"))
}

/// **Halfway, the pixel is both.** A red tile giving way to a blue one over white: at the
/// middle of the fade the centre is neither colour but the two laid over each other, and
/// at the end it is the blue alone.
#[test]
fn halfway_through_a_switch_both_children_are_in_the_pixel() {
    let tile = |value: &u8| {
        Container::<()>::new()
            .width(60.0)
            .height(60.0)
            .color(if *value == 1 {
                Color::rgb(1.0, 0.0, 0.0)
            } else {
                Color::rgb(0.0, 0.0, 1.0)
            })
    };
    let view = |value: u8| {
        Container::<()>::new()
            .width(60.0)
            .height(60.0)
            .color(Color::WHITE)
            .child(AnimatedSwitcher::new(0.2, value, tile))
    };
    let mut stage = Stage::new(60, 60).theme(Theme::dark());
    stage.settle(&view(1));
    stage.advance(&view(2), 0.1);
    let Some(shot) = stage.render(&view(2)) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    let [r, g, b, _] = shot.pixel(30, 30);
    assert!(r > 60 && b > 60, "red and blue both in it: ({r}, {g}, {b})");
    assert!(r < 250 && b < 250, "and neither whole: ({r}, {g}, {b})");

    stage.advance(&view(2), 0.2);
    let shot = stage.render(&view(2)).expect("rendered once already");
    assert_eq!(
        shot.pixel(30, 30)[..3],
        [0, 0, 255],
        "at rest, the blue alone"
    );
}

/// A count going from 3 to 4, caught **halfway**, twice: on the left with the default
/// fade, on the right shrinking away under the new one growing in. Two ends are what a
/// jump also produces; the middle is the argument.
#[test]
fn a_number_changing_matches_its_golden() {
    let figure = |n: &u32| text(n.to_string()).size(40.0).color(Color::WHITE);
    let view = |n: u32| {
        Container::<()>::new().padding(20.0).child(
            Flex::row()
                .gap(40.0)
                .child(AnimatedSwitcher::new(0.2, n, figure))
                .child(
                    AnimatedSwitcher::new(0.2, n, figure).transition(|child, t| {
                        ScaleTransition::new(0.4 + 0.6 * t, FadeTransition::new(t, child))
                    }),
                ),
        )
    };
    let mut stage = Stage::new(180, 100).theme(Theme::dark());
    stage.settle(&view(3));
    stage.advance(&view(4), 0.1);
    let Some(shot) = stage.render(&view(4)) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(shot.lit_pixels(48) > 40, "the frame is empty");
    shot.assert_golden(golden("animated_switcher_halfway"));
}
