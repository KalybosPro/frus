//! The explicit decoration and text-style transitions (milestone 501), **rendered**.
//!
//! The unit tests read the scene. These read the pixels, because the claim that matters
//! most about a decoration in flight is one about colour on the screen: a fill fading out
//! keeps its hue, rather than going dark on the way — and a colour slip is caught by the
//! pixel it produces, not by the number handed to the renderer.

use frus_core::{
    Border, BorderSide, BoxDecoration, BoxShadow, BoxShape, Color, FontWeight, TextStyle,
};
use frus_test::render_widget;
use frus_widgets::{
    text, Container, DecoratedBoxTransition, DefaultTextStyleTransition, Flex, Theme,
};

fn golden(name: &str) -> String {
    format!("{}/tests/goldens/{name}.png", env!("CARGO_MANIFEST_DIR"))
}

/// **A red fill half faded out, over white, is pink — not a darker red.**
///
/// The fill keeps its red and loses half its opacity, so over white the red channel stays
/// full and the other two come up together. Fading towards `Color::TRANSPARENT` instead —
/// transparent *black* — hands the renderer a half-opaque dark red instead, and with that
/// fade put back on purpose this pixel measured (204, 187, 187): a greyed, darker red.
#[test]
fn a_fill_fading_out_over_white_is_pink_not_dark() {
    let tree = Container::<()>::new()
        .width(60.0)
        .height(60.0)
        .color(Color::WHITE)
        .child(DecoratedBoxTransition::between(
            BoxDecoration::filled(Color::rgb(1.0, 0.0, 0.0)),
            BoxDecoration::default(),
            0.5,
            Container::new().width(60.0).height(60.0),
        ));
    let Some(shot) = render_widget(
        &tree,
        60,
        60,
        &Theme::dark().with_platform(frus_core::TargetPlatform::Linux),
    ) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    let [r, g, b, _] = shot.pixel(30, 30);
    assert_eq!(r, 255, "the red is still full red: ({r}, {g}, {b})");
    assert_eq!(g, b, "and the other two came up together: ({r}, {g}, {b})");
    assert!(g > 150, "to a pink, not a dark red: ({r}, {g}, {b})");
}

/// Three moments of each — the start, half-way and the end — because two ends are what a
/// jump also produces.
///
/// Top row: a flat grey tile becoming a rounded card with a border and a shadow. The
/// border thickens in its own colour, the shadow grows out from under the card, and the
/// corners round one by one. Bottom row: a small grey word becoming a large white one,
/// handed down to a text that names neither.
#[test]
fn the_explicit_decoration_and_text_transitions_match_their_golden() {
    let flat = BoxDecoration::filled(Color::rgb(0.35, 0.36, 0.40)).radius(2.0);
    let card = BoxDecoration::filled(Color::rgb(0.20, 0.42, 0.90))
        .radius(18.0)
        .border(Border::new(4.0, Color::rgb(0.75, 0.85, 1.0)))
        .shadow(BoxShadow::new(
            0.0,
            6.0,
            12.0,
            Color::rgba(0.0, 0.0, 0.0, 0.6),
        ));
    let moments = [0.0, 0.5, 1.0];

    let mut tiles = Flex::<()>::row().gap(28.0);
    let mut words = Flex::<()>::row().gap(28.0);
    for t in moments {
        tiles = tiles.child(DecoratedBoxTransition::between(
            flat.clone(),
            card.clone(),
            t,
            Container::new().width(80.0).height(56.0),
        ));
        words = words.child(
            Container::new()
                .width(80.0)
                .height(40.0)
                .child(DefaultTextStyleTransition::between(
                    TextStyle::NONE
                        .size(12.0)
                        .color(Color::rgb(0.55, 0.56, 0.60)),
                    TextStyle::NONE
                        .size(26.0)
                        .color(Color::WHITE)
                        .weight(FontWeight::Bold),
                    t,
                    text("Aa"),
                )),
        );
    }
    let tree = Container::<()>::new()
        .padding(20.0)
        .child(Flex::column().gap(24.0).child(tiles).child(words));

    let Some(shot) = render_widget(
        &tree,
        340,
        180,
        &Theme::dark().with_platform(frus_core::TargetPlatform::Linux),
    ) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(shot.lit_pixels(48) > 40, "the frame is empty");
    shot.assert_golden(golden("explicit_decoration_and_text"));
}

/// **What a box decoration can say, painted** (milestone 641): a card with two shadows, a
/// circle with a ring and a shadow of its own, a box whose sides are four colours, and one
/// whose sides are one colour, uneven, with round corners.
#[test]
fn the_decoration_s_shapes_sides_and_shadows_match_their_golden() {
    let card = Container::<()>::new().width(80.0).height(56.0).decoration(
        BoxDecoration::filled(Color::rgb(0.95, 0.95, 0.97))
            .radius(12.0)
            .shadows([
                BoxShadow::new(0.0, 2.0, 3.0, Color::rgba(0.0, 0.0, 0.0, 0.35)),
                BoxShadow::new(0.0, 8.0, 16.0, Color::rgba(0.0, 0.0, 0.0, 0.25)),
            ]),
    );
    let circle = Container::<()>::new().width(80.0).height(56.0).decoration(
        BoxDecoration::filled(Color::rgb(0.20, 0.42, 0.90))
            .shape(BoxShape::Circle)
            .border(Border::new(3.0, Color::rgb(1.0, 0.85, 0.30)))
            .shadow(BoxShadow::new(
                0.0,
                4.0,
                8.0,
                Color::rgba(0.0, 0.0, 0.0, 0.5),
            )),
    );
    let four = Container::<()>::new()
        .width(80.0)
        .height(56.0)
        .color(Color::rgb(0.18, 0.19, 0.22))
        .box_border(Border {
            top: BorderSide::new(Color::rgb(0.95, 0.30, 0.30), 4.0),
            right: BorderSide::new(Color::rgb(0.30, 0.85, 0.40), 8.0),
            bottom: BorderSide::new(Color::rgb(0.35, 0.55, 1.0), 4.0),
            left: BorderSide::new(Color::rgb(0.95, 0.85, 0.30), 12.0),
        });
    let band = Container::<()>::new()
        .width(80.0)
        .height(56.0)
        .color(Color::rgb(0.18, 0.19, 0.22))
        .radius(14.0)
        .box_border(Border {
            left: BorderSide::new(Color::rgb(0.40, 0.80, 1.0), 10.0),
            ..Border::new(2.0, Color::rgb(0.40, 0.80, 1.0))
        });
    // On a light page, where a black shadow shows.
    let tree = Container::<()>::new()
        .width(460.0)
        .height(120.0)
        .padding(24.0)
        .color(Color::rgb(0.86, 0.87, 0.90))
        .child(
            Flex::row()
                .gap(28.0)
                .child(card)
                .child(circle)
                .child(four)
                .child(band),
        );
    let Some(shot) = render_widget(
        &tree,
        460,
        120,
        &Theme::dark().with_platform(frus_core::TargetPlatform::Linux),
    ) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert!(shot.lit_pixels(48) > 40, "the frame is empty");
    shot.assert_golden(golden("decoration_shapes_sides_shadows"));
}
