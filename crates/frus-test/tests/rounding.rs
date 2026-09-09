//! **Why the layout rounds every box to whole pixels**, measured rather than asserted.
//!
//! Issue #54 offered two shapes and called the first one right: stop rounding, let boxes
//! be fractional, and round at the point of painting — which is what the reference does.
//! One line of taffy turns it off, so the question is not what it costs to write but what
//! it costs to have.
//!
//! This is that measurement. Two fills of different colours abut on a shared edge; the
//! only thing that varies is whether that edge is a whole number.

use frus_core::{Color, Size};
use frus_test::render_widget;
use frus_widgets::{Container, Flex, Theme};

/// The middle column of pixels down a stack of two abutting fills.
fn middle_column(height: f32) -> Option<Vec<(u8, u8, u8)>> {
    // Green behind, so anything of the background that survives between the two fills is
    // unmistakable: neither red nor blue has any green in it at all.
    let tree = Container::<()>::new()
        .width(20.0)
        .height(height)
        .color(Color::rgb(0.0, 1.0, 0.0))
        .child(
            Flex::column()
                .width(20.0)
                .height(height)
                .child(Container::new().flex(1.0).color(Color::rgb(1.0, 0.0, 0.0)))
                .child(Container::new().flex(1.0).color(Color::rgb(0.0, 0.0, 1.0))),
        );
    let shot = render_widget(&tree, 20, height as u32, &Theme::dark())?;
    let _ = Size::new(0.0, 0.0);
    Some(
        (0..height as usize)
            .map(|y| {
                let i = (y * 20 + 10) * 4;
                (shot.rgba[i], shot.rgba[i + 1], shot.rgba[i + 2])
            })
            .collect(),
    )
}

/// **Two boxes that abut meet on a whole pixel, and nothing shows between them.**
///
/// An odd height split in two is the case: 81 into two flex children is 40.5 each, which
/// the engine rounds so that one gets 41 and the other 40 and the seam falls on 41 —
/// where red stops and blue starts, with nothing in between.
///
/// Turn the rounding off and the same edge lands at 40.5. Both fills then cover half of
/// pixel 40 and are composited in turn, so what comes out is a quarter background, a
/// quarter red and a half blue: a **bright green hairline** across an interface that has
/// no green in it. Measured, at the time this was written, as `(136, 187, 136)`.
///
/// That is the whole argument for keeping the rounding, and the reason the rule in
/// `frus_core::fits` exists rather than the rounding being removed. A framework that
/// paints independent anti-aliased quads over a background cannot have fractional box
/// edges for free; the reference gets away with it by compositing differently.
#[test]
fn two_abutting_fills_leave_nothing_between_them() {
    let Some(column) = middle_column(81.0) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    assert_eq!(
        column[40],
        (255, 0, 0),
        "the row above the seam is the first fill, whole"
    );
    assert_eq!(
        column[41],
        (0, 0, 255),
        "and the row below it is the second, whole — no background survives between them"
    );
}

/// The same edge, everywhere down a column of many. Nothing green anywhere is the claim,
/// and one seam in twenty rows would be a bug nobody would find from a single sample.
#[test]
fn no_row_of_a_column_shows_the_background_through_its_edge() {
    let Some(column) = middle_column(199.0) else {
        eprintln!("no GPU adapter available: test skipped");
        return;
    };
    for (y, (_, g, _)) in column.iter().enumerate() {
        assert_eq!(*g, 0, "the background shows through at row {y}");
    }
}
