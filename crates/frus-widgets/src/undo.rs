//! **Undo and redo for a text field**: the history, and the rule that decides what one
//! step is.
//!
//! Ctrl+Z is the shortcut people press without deciding to, and its absence is felt as the
//! field being broken rather than as a feature being missing. What makes it useful rather
//! than merely present is not the stack — that part is fifteen lines — but the answer to
//! *what counts as one step*, because a history that steps back one character at a time is
//! nobody's idea of undo.
//!
//! ## What one step is
//!
//! A **run** of the same kind of change, ended by anything that is not more of the same:
//!
//! - **Typing** joins the run in progress, so a word is one step. A character that is not
//!   a word character — a space, a full stop — joins the run *and closes it behind
//!   itself*, which is what makes undo step back word by word rather than sentence by
//!   sentence.
//! - **Deleting** is a run of its own. A held backspace is one step, and it does not join
//!   the typing it is undoing by hand.
//! - **A pause** of half a second breaks a run, whatever it was. That number is the
//!   reference's, chosen there as the best fit for what Windows, macOS and Linux each do
//!   on their own; there is no better argument for it than that, and no worse one.
//! - **A chunk** — a paste, a cut, a selection replaced by one keystroke, a line break —
//!   is always its own step, and joins nothing on either side. This is the issue's own
//!   list, and they have one thing in common: each is a single deliberate act whose result
//!   the user can see all of.
//! - **A composition** — the provisional text an input method underlines while a word is
//!   being chosen — is one step for the whole word. The clock does not apply to it: the
//!   rhythm of an IME's revisions is the IME's, not the typist's, and a slow suggestion
//!   list is not a pause in the writing.
//! - **Moving the caret closes the run.** Clicking somewhere else, an arrow key, a
//!   select-all: nothing is recorded, because nothing changed, but what is typed next is a
//!   new step. Typing here, then there, then Ctrl+Z should not undo both at once.
//!
//! ## What a step is made of
//!
//! The value **and** the caret, because an undo that restores the text and leaves the
//! caret at the end has only done half the job — the point of undoing is to carry on from
//! where you were.
//!
//! ## What is recorded, and when
//!
//! Entries are the states the field was in **before** each step, and one is pushed only
//! once the value has actually changed. Not when a key that usually changes it arrives:
//! a filter may refuse it, a length limit may swallow it, a read-only field may ignore it,
//! and an Enter may submit instead of typing. The framework records on the evidence of a
//! changed value, which is also how the reference does it — its history listens to the
//! value rather than to the keyboard.

use crate::interaction::Key;
use crate::runtime::Edit;

/// How long a run of typing survives without a keystroke, in seconds.
///
/// The reference's `_kThrottleDuration`, with the reference's own reasoning: a best fit
/// for three platforms that each do something slightly different, and perfect for none.
pub const RUN_PAUSE: f32 = 0.5;

/// The most steps one field keeps.
///
/// The reference's stack is unbounded, which for whole copies of a document is unbounded
/// memory. This is a depth nobody reaches by hand — a hundred and twenty-eight separate
/// deliberate acts in one field — and the oldest entry falls off the bottom when it is.
const DEPTH: usize = 128;

/// A field's value and where the caret was in it: what one step of undo restores.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EditSnapshot {
    /// The whole value.
    pub value: String,
    /// The caret, the selection anchor, and any composing region.
    pub edit: Edit,
}

impl EditSnapshot {
    /// A snapshot of `value` with the caret state `edit`.
    pub fn new(value: impl Into<String>, edit: Edit) -> Self {
        Self {
            value: value.into(),
            edit,
        }
    }
}

/// What sort of change a keystroke made — the unit undo steps back by.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EditKind {
    /// One character typed. `boundary` is a character that ends a word, which joins the
    /// run it is the end of and closes it.
    Typing {
        /// Whether this character ends a word.
        boundary: bool,
    },
    /// A backspace or a forward delete, with nothing selected.
    Deleting,
    /// A revision of the text an input method is still composing.
    Composing,
    /// A paste, a cut, a selection replaced, a line break: one deliberate act, one step.
    Chunk,
}

impl EditKind {
    /// Reads a keystroke as one of the four, given what the field was in when it arrived:
    /// whether something was **selected**, and whether an input method was **composing**.
    ///
    /// The two conditions are the reason this is not a match on the key alone. A backspace
    /// with a selection removes a passage and is a chunk; the same backspace with no
    /// selection removes a letter and joins the run. Typing with a selection replaces it,
    /// which is the case the issue names — a whole selection replaced by one character is
    /// one step, and the step has to include what was replaced.
    pub fn of(key: &Key, selected: bool, composing: bool) -> Self {
        if composing {
            return EditKind::Composing;
        }
        match key {
            Key::Text(text) if !selected && text.chars().count() == 1 => {
                let c = text.chars().next().expect("one character");
                EditKind::Typing {
                    boundary: !c.is_alphanumeric(),
                }
            }
            Key::Backspace | Key::Delete if !selected => EditKind::Deleting,
            _ => EditKind::Chunk,
        }
    }

    /// Whether a change of this kind may join a run of `previous`, `since` seconds after
    /// the last one.
    fn joins(self, previous: EditKind, since: f32) -> bool {
        match (previous, self) {
            // A composition is one act however long the input method takes over it.
            (EditKind::Composing, EditKind::Composing) => true,
            (EditKind::Typing { .. }, EditKind::Typing { .. })
            | (EditKind::Deleting, EditKind::Deleting) => since <= RUN_PAUSE,
            _ => false,
        }
    }

    /// Whether a run may carry on **after** a change of this kind. A word boundary and a
    /// chunk both close the run they end.
    fn leaves_run_open(self) -> bool {
        !matches!(self, EditKind::Typing { boundary: true } | EditKind::Chunk)
    }
}

/// One field's undo history: what it was, and what it was about to be.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UndoHistory {
    /// The states before each step, oldest first.
    past: Vec<EditSnapshot>,
    /// The states undone out of, most recent last.
    future: Vec<EditSnapshot>,
    /// The run in progress, if anything may still join it.
    run: Option<EditKind>,
}

impl UndoHistory {
    /// Records that the field, which **was** `before`, has just been changed by a change
    /// of kind `kind`, `since` seconds after the last one.
    ///
    /// A change that joins the run in progress records nothing: the run's first entry
    /// already holds the state the whole run will be undone to.
    pub fn record(&mut self, before: EditSnapshot, kind: EditKind, since: f32) {
        let joins = self.run.is_some_and(|run| kind.joins(run, since));
        // A new change makes anything that was undone unreachable — it is the branch that
        // was not taken, and there is no way back to it that is not a second history.
        self.future.clear();
        if !joins && self.past.last() != Some(&before) {
            self.past.push(before);
            if self.past.len() > DEPTH {
                self.past.remove(0);
            }
        }
        self.run = kind.leaves_run_open().then_some(kind);
    }

    /// Ends the run in progress without recording anything: the caret moved, so what is
    /// typed next is a step of its own.
    pub fn close_run(&mut self) {
        self.run = None;
    }

    /// Steps back: `current` is where the field is now, and the answer is where it should
    /// go. `None` when there is nothing to undo.
    pub fn undo(&mut self, current: EditSnapshot) -> Option<EditSnapshot> {
        let previous = self.past.pop()?;
        self.future.push(current);
        // An undo ends the run it stepped out of, or the next keystroke would be recorded
        // as more of a step that is no longer there.
        self.run = None;
        Some(previous)
    }

    /// Steps forward again, undoing an undo. `None` when nothing was undone.
    pub fn redo(&mut self, current: EditSnapshot) -> Option<EditSnapshot> {
        let next = self.future.pop()?;
        self.past.push(current);
        self.run = None;
        Some(next)
    }

    /// Whether there is a step to go back to.
    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    /// Whether there is a step to go forward to.
    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A caret at `cursor`, nothing selected.
    fn at(cursor: usize) -> Edit {
        Edit {
            cursor,
            anchor: None,
            composing: None,
        }
    }

    /// Types `text` into `history` one character at a time, the way the shell does:
    /// snapshot what the field was, then record the kind the key reads as.
    fn typed(history: &mut UndoHistory, value: &mut String, text: &str, pause: f32) {
        for c in text.chars() {
            let before = EditSnapshot::new(value.clone(), at(value.chars().count()));
            let key = Key::Text(c.to_string());
            value.push(c);
            history.record(before, EditKind::of(&key, false, false), pause);
        }
    }

    /// The issue's own condition for done: a word, then a second, then Ctrl+Z leaves the
    /// first — **and puts the caret at its end**, which is the half that makes it useful.
    #[test]
    fn a_word_is_one_step_and_undo_brings_the_caret_with_it() {
        let mut history = UndoHistory::default();
        let mut value = String::new();
        typed(&mut history, &mut value, "hello world", 0.05);
        assert_eq!(value, "hello world");

        let now = EditSnapshot::new(value.clone(), at(11));
        let back = history.undo(now.clone()).expect("a step to go back to");
        assert_eq!(
            back.value, "hello ",
            "the second word goes, the first stays"
        );
        assert_eq!(back.edit.cursor, 6, "and the caret is where it was");

        // And back again: the space closed the first run, so "hello " is a step too.
        let back2 = history.undo(back.clone()).expect("the first word");
        assert_eq!(back2.value, "");

        // Redo returns them in order.
        assert_eq!(history.redo(back2).expect("forward").value, "hello ");
        assert_eq!(history.redo(back).expect("forward").value, now.value);
        assert!(!history.can_redo(), "and then there is nothing ahead");
    }

    /// A pause breaks a run even in the middle of a word, because somebody who stopped
    /// typing for half a second and carried on was doing two things.
    #[test]
    fn a_pause_breaks_a_run() {
        let mut history = UndoHistory::default();
        let mut value = String::new();
        typed(&mut history, &mut value, "abc", 0.05);
        // The pause is measured before the first of the second run; the rest is quick.
        let before = EditSnapshot::new(value.clone(), at(3));
        value.push('d');
        history.record(
            before,
            EditKind::Typing { boundary: false },
            RUN_PAUSE + 0.01,
        );
        typed(&mut history, &mut value, "ef", 0.05);
        let now = EditSnapshot::new(value.clone(), at(6));
        assert_eq!(
            history.undo(now).expect("a step").value,
            "abc",
            "the run started again after the pause"
        );
    }

    /// The three the issue names, each one step: a paste, a cut, and a selection replaced
    /// by a keystroke.
    #[test]
    fn a_paste_a_cut_and_a_replacement_are_one_step_each() {
        let paste = Key::Text("a whole clipboard".to_string());
        assert_eq!(EditKind::of(&paste, false, false), EditKind::Chunk);
        // A cut reaches the field as a backspace over a selection.
        assert_eq!(
            EditKind::of(&Key::Backspace, true, false),
            EditKind::Chunk,
            "a deletion with something selected is not a run of backspaces"
        );
        // One character over a selection replaces the lot.
        assert_eq!(
            EditKind::of(&Key::Text("x".to_string()), true, false),
            EditKind::Chunk
        );
        // The same character with nothing selected is ordinary typing.
        assert_eq!(
            EditKind::of(&Key::Text("x".to_string()), false, false),
            EditKind::Typing { boundary: false }
        );
        // And they do not join each other, however fast they arrive.
        let mut history = UndoHistory::default();
        history.record(EditSnapshot::new("one", at(3)), EditKind::Chunk, 0.0);
        history.record(EditSnapshot::new("two", at(3)), EditKind::Chunk, 0.0);
        assert_eq!(history.undo(EditSnapshot::default()).unwrap().value, "two");
        assert_eq!(history.undo(EditSnapshot::default()).unwrap().value, "one");
    }

    /// Deleting is its own run, and does not join the typing it is taking back.
    #[test]
    fn deleting_does_not_join_typing() {
        let mut history = UndoHistory::default();
        let mut value = String::new();
        typed(&mut history, &mut value, "abcd", 0.05);
        // Two backspaces, quickly: one step between them, not two.
        for _ in 0..2 {
            let before = EditSnapshot::new(value.clone(), at(value.chars().count()));
            value.pop();
            history.record(before, EditKind::Deleting, 0.05);
        }
        assert_eq!(value, "ab");
        let back = history
            .undo(EditSnapshot::new(value, at(2)))
            .expect("a step");
        assert_eq!(back.value, "abcd", "the whole run of backspaces comes back");
    }

    /// Moving the caret records nothing and ends the run: typing here, then there, then
    /// undo, takes back only what was typed there.
    #[test]
    fn a_caret_move_closes_the_run() {
        let mut history = UndoHistory::default();
        let mut value = String::from("ab");
        typed(&mut history, &mut value, "cd", 0.05);
        history.close_run(); // the caret moved
        let before = EditSnapshot::new(value.clone(), at(4));
        value.push('e');
        history.record(before, EditKind::Typing { boundary: false }, 0.05);
        let back = history
            .undo(EditSnapshot::new(value, at(5)))
            .expect("a step");
        assert_eq!(back.value, "abcd", "only what was typed after the move");
    }

    /// An input method's revisions are one step for the whole word, and the clock does not
    /// apply to them: a slow suggestion list is not a pause in the writing.
    #[test]
    fn a_composition_is_one_step_however_slow() {
        let mut history = UndoHistory::default();
        history.record(
            EditSnapshot::new("", at(0)),
            EditKind::of(&Key::Text("n".into()), false, true),
            0.0,
        );
        history.record(
            EditSnapshot::new("n", at(1)),
            EditKind::Composing,
            RUN_PAUSE * 10.0,
        );
        history.record(EditSnapshot::new("ni", at(2)), EditKind::Composing, 3.0);
        let back = history
            .undo(EditSnapshot::new("\u{4f60}", at(1)))
            .expect("a step");
        assert_eq!(back.value, "", "the whole composed word");
        assert!(!history.can_undo(), "and it was one entry, not three");
    }

    /// A new change makes what was undone unreachable — the branch not taken.
    #[test]
    fn typing_after_an_undo_forgets_what_was_ahead() {
        let mut history = UndoHistory::default();
        let mut value = String::new();
        typed(&mut history, &mut value, "one ", 0.05);
        typed(&mut history, &mut value, "two", 0.05);
        let back = history
            .undo(EditSnapshot::new(value, at(7)))
            .expect("a step");
        assert!(history.can_redo());
        let mut value = back.value.clone();
        typed(&mut history, &mut value, "six", 0.05);
        assert!(!history.can_redo(), "'two' is not reachable any more");
    }

    /// The same state twice in a row is one entry: an input method that clears its own
    /// provisional text before committing passes back through the value it started from,
    /// and an undo that lands where it already is looks broken.
    #[test]
    fn the_same_state_is_not_recorded_twice() {
        let mut history = UndoHistory::default();
        let state = EditSnapshot::new("hello", at(5));
        history.record(state.clone(), EditKind::Chunk, 0.0);
        history.record(state.clone(), EditKind::Chunk, 0.0);
        assert_eq!(history.undo(EditSnapshot::default()).unwrap(), state);
        assert!(!history.can_undo());
    }

    /// The oldest step falls off the bottom rather than the history growing without end.
    #[test]
    fn the_history_has_a_floor() {
        let mut history = UndoHistory::default();
        for i in 0..DEPTH + 10 {
            history.record(
                EditSnapshot::new(i.to_string(), at(0)),
                EditKind::Chunk,
                0.0,
            );
        }
        let mut seen = 0;
        while history.undo(EditSnapshot::default()).is_some() {
            seen += 1;
        }
        assert_eq!(seen, DEPTH);
    }
}

/// The rule, driven through a **real field** the way the shell drives it.
///
/// The tests above exercise the history on its own; these put a `TextField` in front of it
/// and go through the same four steps `App::apply_key` goes through — read the value, hand
/// the key to the field, keep the caret it hands back, record on the evidence of a changed
/// value — because everything that decides whether undo is right happens in the seam
/// between those two, and a rule tested only on its own side of the seam is a rule that
/// works in a test.
///
/// What is left uncovered is the shell's own plumbing: that Ctrl+Z reaches this at all,
/// and that the restored value is dispatched. Nothing in this repository can drive a
/// frame, which is the roadmap entry this keeps running into.
#[cfg(test)]
mod field_tests {
    use super::*;
    use crate::interaction::WidgetId;
    use crate::runtime::Runtime;
    use crate::textinput::TextField;
    use crate::widget::Widget;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Changed(String),
    }

    /// An application: the value lives here, as it does in a real one, and a message is
    /// the only way it changes.
    struct App {
        value: String,
        runtime: Runtime,
        id: WidgetId,
        /// Whether the field takes digits only — a stand-in for any field that refuses
        /// some of what is typed at it.
        digits: bool,
    }

    impl App {
        fn new(value: &str) -> Self {
            Self {
                value: value.to_string(),
                runtime: Runtime::default(),
                id: WidgetId::from_u64(1),
                digits: false,
            }
        }

        /// The same application, with a field that only takes digits.
        fn digits_only(mut self) -> Self {
            self.digits = true;
            self
        }

        fn field(&self) -> TextField<Msg> {
            let field = TextField::new(self.value.as_str()).on_input(Msg::Changed);
            if self.digits {
                field.input_filter(|c| c.is_ascii_digit().then_some(c))
            } else {
                field
            }
        }

        fn caret(&self) -> Edit {
            self.runtime
                .edits
                .get(&self.id)
                .copied()
                .unwrap_or_default()
        }

        /// The shell's `apply_key`, with the application's own update in the middle.
        fn key(&mut self, key: Key, since: f32) {
            let field = self.field();
            let was = self.caret();
            let mut edit = was;
            let before = Widget::<Msg>::text_value(&field)
                .map(str::to_owned)
                .expect("a field");
            let message = field.on_edit(&mut edit, &key);
            self.runtime.edits.insert(self.id, edit);
            if let Some(Msg::Changed(value)) = message {
                self.value = value;
            }
            if before != self.value {
                let kind = EditKind::of(&key, was.selection_range().is_some(), false);
                self.runtime
                    .record_edit(self.id, EditSnapshot::new(before, was), kind, since);
            } else {
                self.runtime.close_edit_run(self.id);
            }
        }

        /// Types a word, quickly.
        fn typing(&mut self, text: &str) {
            for c in text.chars() {
                self.key(Key::Text(c.to_string()), 0.05);
            }
        }

        /// The shell's `step_history`.
        fn step(&mut self, forward: bool) -> bool {
            let current = EditSnapshot::new(self.value.clone(), self.caret());
            let target = if forward {
                self.runtime.redo_edit(self.id, current)
            } else {
                self.runtime.undo_edit(self.id, current)
            };
            let Some(target) = target else {
                return false;
            };
            let field = self.field();
            if let Some(Msg::Changed(value)) = field.replace_value(target.value.clone()) {
                self.value = value;
            }
            self.runtime.edits.insert(self.id, target.edit);
            true
        }
    }

    /// The issue's own condition for done, end to end: two words, Ctrl+Z, then redo.
    #[test]
    fn two_words_one_undo_and_the_caret_comes_with_it() {
        let mut app = App::new("");
        app.typing("hello world");
        assert_eq!(app.value, "hello world");

        assert!(app.step(false));
        assert_eq!(app.value, "hello ");
        assert_eq!(app.caret().cursor, 6);

        assert!(app.step(true), "and forward again");
        assert_eq!(app.value, "hello world");
        assert_eq!(app.caret().cursor, 11);
        assert!(!app.step(true), "there is nothing beyond it");
    }

    /// A paste is one step, and so is the selection it replaces.
    #[test]
    fn a_paste_over_a_selection_is_one_step() {
        let mut app = App::new("");
        app.typing("keep");
        // Select the lot, then paste over it.
        app.runtime.edits.insert(
            app.id,
            Edit {
                cursor: 4,
                anchor: Some(0),
                composing: None,
            },
        );
        app.key(Key::Text("a whole clipboard".to_string()), 0.05);
        assert_eq!(app.value, "a whole clipboard");
        assert!(app.step(false));
        assert_eq!(
            app.value, "keep",
            "the paste and what it replaced, together"
        );
        assert_eq!(app.caret().anchor, Some(0), "with the selection it had");
    }

    /// A cut — which reaches the field as a backspace over a selection — is one step, and
    /// does not join the backspaces around it.
    #[test]
    fn a_cut_is_its_own_step() {
        let mut app = App::new("");
        app.typing("one two");
        app.key(Key::Backspace, 0.05); // "one tw"
        app.runtime.edits.insert(
            app.id,
            Edit {
                cursor: 6,
                anchor: Some(4),
                composing: None,
            },
        );
        app.key(Key::Backspace, 0.05); // the cut: "one "
        assert_eq!(app.value, "one ");
        assert!(app.step(false));
        assert_eq!(app.value, "one tw", "the cut alone comes back");
        assert!(app.step(false));
        assert_eq!(app.value, "one two", "and then the backspace before it");
    }

    /// A character the field **refuses** is not a step. This is the one that says why the
    /// history is recorded on the evidence of a changed value rather than on a key that
    /// usually changes one: a filter, a length limit, a read-only field and an Enter that
    /// submits all produce keystrokes that change nothing, and a history that records them
    /// has entries that undo to the state they are already in — which from the outside is
    /// indistinguishable from Ctrl+Z being broken.
    #[test]
    fn a_refused_character_is_not_a_step() {
        let mut app = App::new("").digits_only();
        app.typing("12");
        app.key(Key::Text("x".to_string()), 0.05);
        assert_eq!(app.value, "12", "the letter never lands");
        // Enter is the other half of the same point: with nothing to submit to, it is a
        // keystroke in a field that changes nothing at all.
        app.key(Key::Enter, 0.05);
        assert_eq!(app.value, "12");

        // One step back is the run of digits, and there is nothing behind it: neither the
        // refused letter nor the Enter left an entry of its own.
        assert!(app.step(false));
        assert_eq!(app.value, "");
        assert!(
            !app.step(false),
            "no entry for a keystroke that did nothing"
        );
    }

    /// A read-only field has no history at all: nothing it holds ever came from a
    /// keystroke, and `replace_value` refuses for the same reason `on_edit` does.
    #[test]
    fn a_read_only_field_has_nothing_to_undo() {
        let field: TextField<Msg> = TextField::new("fixed").on_input(Msg::Changed).read_only();
        let mut edit = Edit::default();
        assert!(field
            .on_edit(&mut edit, &Key::Text("x".to_string()))
            .is_none());
        assert!(field.replace_value("something else".to_string()).is_none());
    }
}
