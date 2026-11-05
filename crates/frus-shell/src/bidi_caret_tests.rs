// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 KalybosPro <kalyboskokou1@gmail.com>

//! A field's caret through text that mixes directions, driven through the shell (milestone
//! 747): a click beside a change of direction puts the caret where the pointer is, the
//! arrow keys move it the way they point on screen, and End in a wrapped right-to-left
//! field paints it at the end of the line it is on.

use crate::app::testing::Driver;
use crate::{Application, Command};
use frus_widgets::{
    Container, Flex, KeyStroke, Point, Rect, ShortcutKey, TextField, Theme, Widget,
};

const W: f32 = 400.0;
const H: f32 = 300.0;

/// Latin, Hebrew, Latin: drawn a b c ␠ ג ב א ␠ d e f.
const MIXED: &str = "abc \u{5d0}\u{5d1}\u{5d2} def";

/// Eight Hebrew words, which wrap in a field 200 px wide.
const HEBREW: &str = concat!(
    "\u{5e9}\u{5dc}\u{5d5}\u{5dd} \u{5e2}\u{5d5}\u{5dc}\u{5dd} ",
    "\u{5d6}\u{5d4} \u{5de}\u{5e9}\u{5e4}\u{5d8} \u{5d0}\u{5e8}\u{5d5}\u{5da} ",
    "\u{5e9}\u{5e0}\u{5e9}\u{5d1}\u{5e8} \u{5dc}\u{5db}\u{5de}\u{5d4} ",
    "\u{5e9}\u{5d5}\u{5e8}\u{5d5}\u{5ea}"
);

/// One field, holding `value`: a single line, or a wrapped paragraph four rows tall.
struct Form {
    value: String,
    multiline: bool,
}

impl Application for Form {
    type Message = String;

    fn update(&mut self, value: String) -> Command<String> {
        self.value = value;
        Command::none()
    }

    fn view(&self, _theme: &Theme) -> Box<dyn Widget<String>> {
        let field = TextField::new(self.value.clone()).on_input(|value| value);
        let field = if self.multiline {
            field.multiline().rows(4).width(200.0)
        } else {
            field.width(W - 40.0)
        };
        Box::new(
            Container::new()
                .width(W)
                .height(H)
                .child(Flex::column().child(field)),
        )
    }
}

fn open(value: &str, multiline: bool) -> Driver<Form> {
    let mut driver = Driver::new(
        Form {
            value: value.into(),
            multiline,
        },
        W,
        H,
    );
    driver.run(0.2);
    driver
}

/// The caret painted in the frame: the one rectangle two pixels wide.
fn caret(driver: &Driver<Form>) -> Rect {
    let (ui, _) = driver.frame_parts().expect("a frame");
    let carets: Vec<Rect> = ui
        .scene()
        .primitives()
        .iter()
        .filter_map(|p| match p {
            frus_widgets::Primitive::Rect { rect, .. } if rect.width == 2.0 => Some(*rect),
            _ => None,
        })
        .collect();
    assert_eq!(carets.len(), 1, "one caret: {carets:?}");
    carets[0]
}

/// Where the text that reads `value` was drawn.
fn drawn(driver: &Driver<Form>, value: &str) -> Rect {
    driver
        .texts()
        .into_iter()
        .find(|(text, _)| text == value)
        .unwrap_or_else(|| panic!("{value:?} is on screen: {:?}", driver.texts()))
        .1
}

fn press(driver: &mut Driver<Form>, key: ShortcutKey) {
    driver.key(KeyStroke::new(key));
    driver.run(0.05);
}

fn click(driver: &mut Driver<Form>, at: Point) {
    driver.press_mouse(at);
    driver.run(0.02);
    driver.release_mouse(at);
    driver.run(0.3);
}

/// **A click beside a change of direction, then the arrows**: the caret is painted where
/// the pointer went down, on the left of the last Hebrew letter; Right then moves it right
/// at every press to the end of the line, and Left back to its start.
#[test]
fn the_arrows_move_a_fields_caret_the_way_they_point() {
    let mut d = open(MIXED, false);
    let text = drawn(&d, MIXED);
    let y = text.y + 8.0;
    // The ends of the line, as the field paints them: Home and End.
    click(&mut d, Point::new(text.x + 1.0, y));
    press(&mut d, ShortcutKey::Home);
    let start = caret(&d).x;
    press(&mut d, ShortcutKey::End);
    let end = caret(&d).x;
    assert!(end > start + 50.0, "{start} {end}");
    // The field draws in its own size; the letters keep their proportions, so the gap
    // between "abc " and the Hebrew run is found from the line's painted width.
    let layout = frus_text::TextLayout::in_block(
        MIXED,
        &frus_widgets::TextStyle::NONE.resolved(),
        frus_widgets::TextBlock {
            width: None,
            soft_wrap: true,
            align: frus_widgets::TextAlign::Start,
        },
    );
    let scale = (end - start) / layout.size().width;
    let gimel = layout.selection_rects(6, 7)[0];
    let x = start + (gimel.x + gimel.width * 0.25) * scale;
    click(&mut d, Point::new(x, y));
    let placed = caret(&d);
    assert!(
        (placed.x - x).abs() < gimel.width * 0.5 * scale + 1.0,
        "the caret is where the click was: {placed:?} vs {x}"
    );
    // Right to the end, every press to the right.
    let mut last = placed.x;
    for _ in 0..12 {
        press(&mut d, ShortcutKey::Right);
        let now = caret(&d).x;
        assert!(now >= last, "{last} -> {now}");
        last = now;
    }
    assert!((last - end).abs() < 0.5, "at the end: {last} {end}");
    // Left to the start, every press to the left.
    for _ in 0..12 {
        press(&mut d, ShortcutKey::Left);
        let now = caret(&d).x;
        assert!(now <= last, "{last} -> {now}");
        last = now;
    }
    assert!((last - start).abs() < 0.5, "at the start: {last} {start}");
}

/// **End in a wrapped right-to-left field** paints the caret at the left end of the first
/// line — on that line, not at the start of the next — and Home brings it back to the
/// right end.
#[test]
fn end_stays_on_the_line_of_a_wrapped_right_to_left_field() {
    let mut d = open(HEBREW, true);
    let text = drawn(&d, HEBREW);
    // A click at the far left of the first line puts the caret at its end, on it.
    click(&mut d, Point::new(text.x + 2.0, text.y + 8.0));
    let clicked = caret(&d);
    press(&mut d, ShortcutKey::Home);
    let start = caret(&d);
    assert_eq!(start.y, clicked.y, "Home stays on the line");
    assert!(
        start.x > clicked.x + 50.0,
        "at its right: {clicked:?} -> {start:?}"
    );
    press(&mut d, ShortcutKey::End);
    let end = caret(&d);
    assert_eq!(end.y, start.y, "on the first line: {start:?} -> {end:?}");
    assert!(
        (end.x - clicked.x).abs() < 0.5,
        "at its left: {clicked:?} {end:?}"
    );
    // Down from the end of the first line is on the second.
    press(&mut d, ShortcutKey::Down);
    assert!(caret(&d).y > start.y);
}

/// **A double click is two presses close together**, in time and in place: two quick presses
/// far apart in a field each place the caret and select no word, and two near each other
/// select the word under them (milestone 747). Driven presses are microseconds apart on the
/// wall: timed by it, the first pair selected a word too.
#[test]
fn a_double_click_in_a_field_is_two_presses_close_together() {
    let words = "alpha beta gamma delta";
    let mut d = open(words, false);
    let text = drawn(&d, words);
    let y = text.y + 8.0;
    let layout = frus_text::TextLayout::in_block(
        words,
        &frus_widgets::TextStyle::NONE.resolved(),
        frus_widgets::TextBlock {
            width: None,
            soft_wrap: true,
            align: frus_widgets::TextAlign::Start,
        },
    );
    let middle = |i: usize| {
        let r = layout.selection_rects(i, i + 1)[0];
        Point::new(text.x + r.x + r.width * 0.5, y)
    };
    // "beta" (its 'e'), then at once "delta" (its 'l'): two clicks, no word.
    let (beta, delta) = (middle(7), middle(19));
    d.press_mouse(beta);
    d.release_mouse(beta);
    d.press_mouse(delta);
    d.release_mouse(delta);
    d.run(0.05);
    assert_eq!(d.field_selection(), None, "far apart");
    // Long enough after for a fresh start, then twice on "beta": the word.
    d.run(1.0);
    d.press_mouse(beta);
    d.release_mouse(beta);
    d.run(0.1);
    d.press_mouse(beta);
    d.release_mouse(beta);
    d.run(0.05);
    assert_eq!(d.field_selection(), Some((6, 10)), "close together");
    // And again after the double click's time: a single click, the word let go.
    d.run(1.0);
    d.press_mouse(beta);
    d.release_mouse(beta);
    d.run(0.05);
    assert_eq!(d.field_selection(), None, "too late");
}
