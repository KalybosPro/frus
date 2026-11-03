// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! Selecting aligned and cut texts, driven through the shell (milestone 742): a drag in a
//! centred and in an end-aligned line of a selection area copies the words it went over
//! and paints their highlight under them, a text selectable on its own does the same, and
//! a drag to the end of a text cut by an ellipsis copies the whole of it.

use crate::app::testing::Driver;
use crate::{Application, Command};
use frus_widgets::{
    Container, Flex, KeyStroke, Point, Rect, SelectionArea, ShortcutKey, Text, TextAlign,
    TextOverflow, TextStyle, Theme, Widget,
};

const W: f32 = 400.0;
const H: f32 = 300.0;
const CENTRED: &str = "centred words here";
const ENDED: &str = "ending words here";
const CUT: &str = "a sentence far too long to be drawn whole on one line of this narrow box";

/// Three texts in a selection area — one centred, one end-aligned, one cut by an ellipsis —
/// or a centred one selectable on its own.
struct Doc {
    alone: bool,
}

impl Application for Doc {
    type Message = ();

    fn update(&mut self, _message: ()) -> Command<()> {
        Command::none()
    }

    fn view(&self, _theme: &Theme) -> Box<dyn Widget<()>> {
        let child: Box<dyn Widget<()>> = if self.alone {
            Box::new(Flex::column().child(Text::new(CENTRED).selectable().align(TextAlign::Center)))
        } else {
            Box::new(SelectionArea::around(
                Flex::column()
                    .child(Text::new(CENTRED).align(TextAlign::Center))
                    .child(Text::new(ENDED).align(TextAlign::End))
                    .child(Text::new(CUT).max_lines(1).overflow(TextOverflow::Ellipsis)),
            ))
        };
        Box::new(Container::new().width(W).height(H).child(child))
    }
}

fn driver(alone: bool) -> Driver<Doc> {
    let mut driver = Driver::new(Doc { alone }, W, H);
    driver.run(0.2);
    driver
}

/// The width of `prefix` in the default text style.
fn width(prefix: &str) -> f32 {
    frus_text::measure_resolved(prefix, &TextStyle::NONE.resolved()).width
}

/// The box the text that reads `label` (or starts with it) was drawn at.
fn drawn(driver: &Driver<Doc>, label: &str) -> Rect {
    driver
        .texts()
        .into_iter()
        .find(|(text, _)| text.starts_with(label))
        .unwrap_or_else(|| panic!("{label:?} is on screen: {:?}", driver.texts()))
        .1
}

/// A point on the line of `label`, `across` px from the left of its box.
fn on(driver: &Driver<Doc>, label: &str, across: f32) -> Point {
    let rect = drawn(driver, label);
    Point::new(rect.x + across, rect.y + 8.0)
}

fn drag(driver: &mut Driver<Doc>, from: Point, to: Point) {
    driver.press_mouse(from);
    driver.run(0.05);
    driver.move_mouse(to);
    driver.run(0.05);
    driver.release_mouse(to);
    driver.run(0.05);
}

fn rects(d: &Driver<Doc>) -> Vec<Rect> {
    let (ui, _) = d.frame_parts().expect("a frame");
    ui.scene()
        .primitives()
        .iter()
        .filter_map(|p| match p {
            frus_widgets::Primitive::Rect { rect, .. } => Some(*rect),
            _ => None,
        })
        .collect()
}

fn copy(driver: &mut Driver<Doc>) -> Option<String> {
    driver.key(KeyStroke::new(ShortcutKey::Char('c')).ctrl());
    driver.run(0.05);
    driver.clipboard()
}

/// A drag over `range` of the text's own x coordinates, then what it copied and the one
/// highlight box it added to the frame.
fn select(d: &mut Driver<Doc>, label: &str, from: f32, to: f32) -> (Option<String>, Rect) {
    let bare = rects(d);
    let (a, b) = (on(d, label, from), on(d, label, to));
    drag(d, a, b);
    let copied = copy(d);
    let added: Vec<Rect> = rects(d)
        .into_iter()
        // A text selectable on its own also draws its focus ring, around its whole box.
        .filter(|rect| !bare.contains(rect) && rect.width < W)
        .collect();
    assert_eq!(added.len(), 1, "one highlight: {added:?}");
    (copied, added[0])
}

/// **A drag over a word of a centred line copies that word** and highlights it where it is
/// drawn — half the room left over in from the edge, not at the start of the box.
#[test]
fn a_drag_in_a_centred_line_copies_and_highlights_the_word() {
    let mut d = driver(false);
    let left = (W - width(CENTRED)) / 2.0;
    let start = left + width("centred ");
    let end = left + width("centred words");
    let (copied, highlight) = select(&mut d, CENTRED, start + 1.0, end + 1.0);
    assert_eq!(copied.as_deref(), Some("words"));
    let rect = drawn(&d, CENTRED);
    assert!(
        (highlight.x - (rect.x + start)).abs() < 1.5,
        "{highlight:?} from {start}"
    );
    assert!(
        (highlight.width - width("words")).abs() < 1.5,
        "{highlight:?}"
    );
    assert!(
        (highlight.y - rect.y).abs() < 1.0,
        "{highlight:?} on {rect:?}"
    );
}

/// **A drag over the last word of an end-aligned line copies it**, highlighted against the
/// right edge of the box.
#[test]
fn a_drag_in_an_end_aligned_line_copies_and_highlights_the_word() {
    let mut d = driver(false);
    let start = W - width("here");
    let (copied, highlight) = select(&mut d, ENDED, start + 1.0, W - 1.0);
    assert_eq!(copied.as_deref(), Some("here"));
    let rect = drawn(&d, ENDED);
    assert!(
        (highlight.x + highlight.width - (rect.x + W)).abs() < 1.5,
        "{highlight:?} to the edge"
    );
    assert!(
        (highlight.width - width("here")).abs() < 1.5,
        "{highlight:?}"
    );
}

/// **A drag to the end of a text an ellipsis cut short copies the whole of it**, and its
/// highlight stays on the one line drawn.
#[test]
fn a_drag_to_the_end_of_a_cut_text_copies_all_of_it() {
    let mut d = driver(false);
    let (copied, highlight) = select(&mut d, "a sentence", 1.0, W - 1.0);
    assert_eq!(copied.as_deref(), Some(CUT));
    let rect = drawn(&d, "a sentence");
    assert!(
        highlight.x + highlight.width <= rect.x + W + 0.5,
        "{highlight:?}"
    );
    assert!((highlight.y - rect.y).abs() < 1.0, "{highlight:?}");
}

/// **A centred text selectable on its own** selects the word a drag went over and copies
/// it with Ctrl+C.
#[test]
fn a_selectable_centred_text_copies_what_was_dragged_over() {
    let mut d = driver(true);
    let left = (W - width(CENTRED)) / 2.0;
    let start = left + width("centred ");
    let end = left + width("centred words");
    let (copied, highlight) = select(&mut d, CENTRED, start + 1.0, end + 1.0);
    assert_eq!(copied.as_deref(), Some("words"));
    let rect = drawn(&d, CENTRED);
    assert!(
        (highlight.x - (rect.x + start)).abs() < 1.5,
        "{highlight:?} from {start}"
    );
}
