//! What an input method's operations do to a field (milestone 510).
//!
//! A keyboard that predicts — SwiftKey, Gboard — writes a word as a **composition**:
//! provisional text it replaces on every keystroke (`F`, then `Fr`, then `Fru`) until it
//! finishes it. The shell used to track that composition as a *count* of characters and
//! erase it with that many backspaces from the caret. The count lived on the shell, and the
//! paths that ask for the keyboard again zeroed it — so on a device the next update erased
//! nothing and wrote the word again beside itself: `F`, then `FFr`. A word the keyboard
//! reclaimed after finishing it (`setComposingRegion`) was not forwarded at all, and came
//! back with the next letter as a second copy.
//!
//! Now the composition is the **field's own** — [`Edit::composing`], the range the field
//! already underlines — and every operation is planned against it: select the range, type
//! over it. [`plan`] turns one operation into steps the shell carries out through the field's
//! ordinary editing, which is why it can be tested here, away from a device.

use frus_widgets::{Edit, Key};

/// An input operation relayed by the IME; arrival order is preserved.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ImeEvent {
    /// **Final** text: a plain keystroke, a swipe, a chosen suggestion, an emoji.
    Commit(String),
    /// Text **being composed**, replacing the previous composition.
    Composing(String),
    /// Text already in the field becomes the composition — a keyboard reclaiming a word it
    /// finished, to go on predicting it. In **UTF-16 units**, as Java counts.
    ComposingRegion { start: u32, end: u32 },
    /// The current composition becomes final, as it stands.
    FinishComposing,
    /// Deletes `before` characters ahead of the cursor and `after` behind it.
    Delete { before: u32, after: u32 },
    /// An editor action: the keyboard's Enter, OK or Search.
    Action,
    /// A key relayed by the IME (`sendKeyEvent`), already filtered on the Java side.
    Key { code: i32, unicode: u32 },
}

/// One thing to do to the focused field, in order.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Step {
    /// Put the caret at `cursor`, the selection anchored at `anchor`.
    Place {
        cursor: usize,
        anchor: Option<usize>,
    },
    /// An ordinary key, through the field's own editing.
    Key(Key),
    /// The composing range, as given — `None` ends the composition.
    Compose(Option<(usize, usize)>),
    /// The composing range from `start` to **wherever the caret ended up**, so that a
    /// character the field's filter refused is not counted as composed.
    ComposeTo(usize),
}

/// A position in UTF-16 units — as Java counts — as a position in characters.
pub(crate) fn char_index(text: &str, utf16: usize) -> usize {
    let mut units = 0;
    for (index, c) in text.chars().enumerate() {
        if units >= utf16 {
            return index;
        }
        units += c.len_utf16();
    }
    text.chars().count()
}

/// A position in characters as a position in UTF-16 units, for Java.
pub(crate) fn utf16_index(text: &str, chars: usize) -> usize {
    text.chars().take(chars).map(char::len_utf16).sum()
}

/// Types `with` over the composition — or over the selection, or at the caret, when there
/// is none, which is what `commitText` and `setComposingText` promise. Returns the steps
/// and where the new text starts.
fn type_over(edit: &Edit, len: usize, with: &str) -> (Vec<Step>, usize) {
    let composing = edit
        .composing
        .map(|(s, e)| (s.min(len), e.min(len)))
        .filter(|(s, e)| s < e);
    let mut steps = Vec::new();
    let start = match composing {
        Some((s, e)) => {
            steps.push(Step::Place {
                cursor: e,
                anchor: Some(s),
            });
            s
        }
        // A key replaces the selection by itself.
        None => edit
            .selection_range()
            .map(|(s, _)| s)
            .unwrap_or(edit.cursor.min(len)),
    };
    if !with.is_empty() {
        steps.push(Step::Key(Key::Text(with.to_string())));
    } else if composing.is_some() || edit.selection_range().is_some() {
        steps.push(Step::Key(Key::Backspace));
    }
    (steps, start)
}

/// What `event` does to a field holding `text`, whose edit state is `edit`.
pub(crate) fn plan(event: &ImeEvent, edit: &Edit, text: &str) -> Vec<Step> {
    let len = text.chars().count();
    match event {
        // A `\n` commit, which some IMEs send, means submit, not insert.
        ImeEvent::Commit(with) if with == "\n" || with == "\r" => {
            let (mut steps, _) = type_over(edit, len, "");
            steps.push(Step::Compose(None));
            steps.push(Step::Key(Key::Enter));
            steps
        }
        ImeEvent::Commit(with) => {
            let (mut steps, _) = type_over(edit, len, with);
            steps.push(Step::Compose(None));
            steps
        }
        ImeEvent::Composing(with) => {
            let (mut steps, start) = type_over(edit, len, with);
            steps.push(if with.is_empty() {
                Step::Compose(None)
            } else {
                Step::ComposeTo(start)
            });
            steps
        }
        ImeEvent::ComposingRegion { start, end } => {
            let a = char_index(text, *start as usize);
            let b = char_index(text, *end as usize);
            let (s, e) = (a.min(b), a.max(b));
            vec![Step::Compose((s < e).then_some((s, e)))]
        }
        ImeEvent::FinishComposing => vec![Step::Compose(None)],
        ImeEvent::Delete { before, after } => {
            let mut steps = vec![Step::Key(Key::Backspace); *before as usize];
            steps.extend(vec![Step::Key(Key::Delete); *after as usize]);
            steps
        }
        ImeEvent::Action => vec![Step::Key(Key::Enter)],
        ImeEvent::Key { code, unicode } => match code {
            66 => vec![Step::Key(Key::Enter)],
            67 => vec![Step::Key(Key::Backspace)],
            _ => char::from_u32(*unicode)
                .filter(|c| !c.is_control())
                .map(|c| vec![Step::Key(Key::Text(c.to_string()))])
                .unwrap_or_default(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The field, as far as an input method's operations can tell: its characters, its
    /// edit state and how many times it was submitted. It edits the way `TextField` does —
    /// a key replaces the selection, Backspace takes the selection or the character before
    /// the caret — with an optional filter that refuses digits.
    #[derive(Default)]
    struct Field {
        chars: Vec<char>,
        edit: Edit,
        submitted: usize,
        no_digits: bool,
    }

    impl Field {
        fn text(&self) -> String {
            self.chars.iter().collect()
        }

        fn run(&mut self, event: ImeEvent) {
            let text = self.text();
            for step in plan(&event, &self.edit, &text) {
                match step {
                    Step::Place { cursor, anchor } => {
                        self.edit.cursor = cursor;
                        self.edit.anchor = anchor;
                    }
                    Step::Key(key) => self.key(key),
                    Step::Compose(region) => self.edit.composing = region,
                    Step::ComposeTo(start) => {
                        let end = self.edit.cursor;
                        self.edit.composing = (end > start).then_some((start, end));
                    }
                }
            }
        }

        fn key(&mut self, key: Key) {
            let len = self.chars.len();
            let mut cursor = self.edit.cursor.min(len);
            let selection = self.edit.selection_range();
            match key {
                Key::Text(text) => {
                    if let Some((s, e)) = selection {
                        self.chars.drain(s..e);
                        cursor = s;
                    }
                    let typed: Vec<char> = text
                        .chars()
                        .filter(|c| !(self.no_digits && c.is_ascii_digit()))
                        .collect();
                    for c in typed {
                        self.chars.insert(cursor, c);
                        cursor += 1;
                    }
                }
                Key::Backspace => {
                    if let Some((s, e)) = selection {
                        self.chars.drain(s..e);
                        cursor = s;
                    } else if cursor > 0 {
                        self.chars.remove(cursor - 1);
                        cursor -= 1;
                    }
                }
                Key::Delete => {
                    if let Some((s, e)) = selection {
                        self.chars.drain(s..e);
                        cursor = s;
                    } else if cursor < len {
                        self.chars.remove(cursor);
                    }
                }
                Key::Enter => self.submitted += 1,
                _ => {}
            }
            self.edit.cursor = cursor;
            self.edit.anchor = None;
        }
    }

    fn composing(text: &str) -> ImeEvent {
        ImeEvent::Composing(text.into())
    }

    /// **Each composition is typed over the one before, not beside it** (milestone 510).
    /// `F` then `Fr` is `Fr`. On the device it was `FFr`: the composition was a count the
    /// shell kept, zeroed between the two updates, so the second erased nothing.
    #[test]
    fn each_composition_replaces_the_one_before() {
        let mut field = Field::default();
        field.run(composing("F"));
        field.run(composing("Fr"));
        assert_eq!(field.text(), "Fr");
        assert_eq!(
            field.edit.composing,
            Some((0, 2)),
            "and it is still being composed"
        );
    }

    /// A composition after text already written starts at the caret, and leaves what was
    /// there alone.
    #[test]
    fn a_composition_starts_at_the_caret_after_what_is_written() {
        let mut field = Field::default();
        field.run(ImeEvent::Commit("Hi ".into()));
        field.run(composing("t"));
        field.run(composing("th"));
        assert_eq!(field.text(), "Hi th");
        assert_eq!(field.edit.composing, Some((3, 5)));
    }

    #[test]
    fn a_commit_replaces_the_composition() {
        let mut field = Field::default();
        field.run(composing("helo"));
        field.run(ImeEvent::Commit("hello".into()));
        assert_eq!(field.text(), "hello");
        assert_eq!(field.edit.composing, None);
        assert_eq!(field.edit.cursor, 5);
    }

    /// **A word the keyboard reclaims is typed over.** After finishing `Fr`, the keyboard
    /// on the device marked it as composing again and sent it back with the next letter.
    /// Not forwarded, the region stayed empty and `Fru` was written beside `Fr`. This is
    /// the device's own sequence, as a field that follows it should end.
    #[test]
    fn a_word_the_keyboard_reclaims_is_typed_over() {
        let mut field = Field::default();
        for event in [
            composing("F"),
            composing("Fr"),
            ImeEvent::FinishComposing,
            ImeEvent::ComposingRegion { start: 0, end: 2 },
            composing("Fru"),
            composing("Frus"),
            composing("Frusc"),
            composing("Fruscl"),
            composing("Frusclip"),
        ] {
            field.run(event);
        }
        assert_eq!(field.text(), "Frusclip");
    }

    #[test]
    fn finishing_keeps_the_text_and_ends_the_composition() {
        let mut field = Field::default();
        field.run(composing("word"));
        field.run(ImeEvent::FinishComposing);
        assert_eq!(field.text(), "word");
        assert_eq!(field.edit.composing, None);
        // And the next composition starts after it rather than over it.
        field.run(composing("s"));
        assert_eq!(field.text(), "words");
    }

    #[test]
    fn an_empty_composition_takes_the_composed_text_away() {
        let mut field = Field::default();
        field.run(composing("abc"));
        field.run(composing(""));
        assert_eq!(field.text(), "");
        assert_eq!(field.edit.composing, None);
    }

    /// A `\n` commit submits rather than types, and leaves a finished word where it is.
    #[test]
    fn a_newline_commit_submits_rather_than_types() {
        let mut field = Field::default();
        field.run(composing("go"));
        field.run(ImeEvent::FinishComposing);
        field.run(ImeEvent::Commit("\n".into()));
        assert_eq!(field.text(), "go");
        assert_eq!(field.submitted, 1);
    }

    /// **A character the field refuses is not counted as composed.** A filter that drops
    /// digits leaves `a` of `a1`; counting the composition from the text sent would claim
    /// two characters and take the next one with it.
    #[test]
    fn a_refused_character_is_not_counted_as_composed() {
        let mut field = Field {
            no_digits: true,
            ..Field::default()
        };
        field.run(ImeEvent::Commit("x".into()));
        field.run(composing("a1"));
        assert_eq!(field.text(), "xa");
        assert_eq!(field.edit.composing, Some((1, 2)));
        field.run(composing("a1b"));
        assert_eq!(field.text(), "xab", "the x before it survives");
    }

    /// The keys and actions a keyboard sends instead of text are the field's own keys:
    /// Backspace and Enter by their codes, a character as text, a control character as
    /// nothing, and a surrounding deletion as that many Backspaces and Deletes.
    #[test]
    fn keys_and_actions_are_the_fields_own_keys() {
        let mut field = Field::default();
        field.run(ImeEvent::Commit("abcd".into()));
        field.run(ImeEvent::Key {
            code: 67,
            unicode: 0,
        });
        assert_eq!(field.text(), "abc", "67 is Backspace");
        field.run(ImeEvent::Key {
            code: 29,
            unicode: 'x' as u32,
        });
        assert_eq!(field.text(), "abcx", "a character is typed");
        field.run(ImeEvent::Key {
            code: 61,
            unicode: '\t' as u32,
        });
        assert_eq!(field.text(), "abcx", "a control character is not");
        field.edit.cursor = 2;
        field.run(ImeEvent::Delete {
            before: 1,
            after: 1,
        });
        assert_eq!(field.text(), "ax", "one before the caret, one after");
        field.run(ImeEvent::Action);
        field.run(ImeEvent::Key {
            code: 66,
            unicode: 0,
        });
        assert_eq!(field.submitted, 2, "the action and 66 are Enter");
    }

    /// Java counts in UTF-16 units, the field in characters: an emoji is two of one and one
    /// of the other, and a region given past it has to land on the right characters.
    #[test]
    fn positions_are_converted_from_the_units_java_counts_in() {
        let text = "a\u{1F600}b";
        assert_eq!(char_index(text, 3), 2);
        assert_eq!(utf16_index(text, 2), 3);
        let mut field = Field::default();
        field.run(ImeEvent::Commit(text.into()));
        field.run(ImeEvent::ComposingRegion { start: 3, end: 4 });
        assert_eq!(
            field.edit.composing,
            Some((2, 3)),
            "the b, not half the emoji"
        );
        // A region given backwards is the same region.
        field.run(ImeEvent::ComposingRegion { start: 4, end: 3 });
        assert_eq!(field.edit.composing, Some((2, 3)));
    }
}
