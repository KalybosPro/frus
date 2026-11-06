# Milestone 764 — Words on screen, and a caret beside what was typed

## Objective

Milestone 747 gave a caret an affinity and made the arrows move it the way they point through a
line that mixes directions. Its "Not done" left two editing gaps at a change of direction:

- **Ctrl and an arrow** still moved a word at a time through the text's order. In
  `abc אבג def` (drawn `abc גבא def`), Ctrl+Right after `abc` crossed the Hebrew word in the
  text's order and left the caret after gimel — drawn at the run's **left**, just after the
  space it had started beside. The press seemed to move the caret one space and skip nothing.
- **Typing at a change of direction** always left the caret downstream, before what follows. A
  Latin `x` typed with the caret at the right of the Hebrew run (before alef) goes, as it must,
  logically before alef — which draws it after the space, at the run's left — and the caret
  stayed at the right of the run, a whole word away from the letter just typed.

Milestone 760 also left one: no golden renders a rich paragraph above 1×, so the letter-spacing
scaling bug it fixed could come back unseen.

## What was done

- **Ctrl and an arrow move a word at a time on screen.** With the text's layout (a field, a
  selectable `Text`, a selectable `RichText`), the caret crosses, one drawn character at a time,
  whatever separates it from the next word on that side, then that word, and stops beside its
  last letter on screen. Through `abc אבג def`, Ctrl+Right goes after `abc`, then to the right
  of the Hebrew word (before alef), then after `def`; Ctrl+Left comes back before `def`, to the
  left of the Hebrew word (after gimel), then to the start. Every press moves the painted caret
  the way the arrow points.
  On a line in one direction this is exactly the word movement through the text's order it
  replaces — forwards it stops after a word, back before one, and in Hebrew Left is forwards —
  which a test checks from every boundary of a Latin and a Hebrew line. The end of a line counts
  as a separator, so in a wrapped field the caret goes on to the next line the way the arrows
  do. Without a layout (a widget that does not answer `caret_key`) the movement is through the
  text's order, as before.
- **Typing leaves the caret beside what was typed.** After text is typed or pasted, the caret
  belongs to its last character (upstream), so at a change of direction it is drawn right after
  that character, wherever the bidirectional algorithm put it. In text of one direction the two
  sides of a boundary are one place, so nothing changes there; at a soft wrap the caret stays on
  the line the typed character ended up on. After a line break, a deletion or an Enter, the
  caret is before what follows it, as before.
- **A golden of spaced rich text selected on a dense screen.** `frus-test`
  `spaced_rich_text_selected_on_a_dense_screen_matches_its_golden` paints a `RichText` with a
  letter spacing of 1 and the word "framework" selected, scales the scene by 2.625 as the shell
  scales a phone's, and renders it (`rich_text_spaced_selection_2_625x.png`). This branch merges
  `fix/selection-highlight-offset` (milestone 760, PR #413, not yet merged when this was
  written), so the golden shows the fixed rendering: the highlight sits under its word.

## Decisions

**Where the caret goes, not where the text goes.** What is typed is inserted at the caret's
offset, whatever its side. Choosing the offset by the side — putting a Hebrew letter typed with
the caret drawn after the space in `abc |אבג` logically after gimel, so that it appears where
the caret was — would make the same keystrokes produce different text depending on how the
caret arrived, and the platforms insert at the offset too. What the side does decide is where
the caret is drawn **afterwards**, and that is now beside the text just typed.

**Visual words, same stops.** A word on screen is found by walking the same visual steps the
arrows take and classifying each crossed character as a word character or not (letters, digits
and `_`, as before). On a line in one direction the stops are the logical ones, so nothing
changes for text that does not mix directions, and a mixed line moves the way it is drawn. The
selection under Shift stays a range of the text's order, as for the arrows.

## Tests

The new tests of the field fail before the change:

- `textinput::bidi_tests::ctrl_and_an_arrow_cross_the_word_drawn_next_to_the_caret`: through
  `abc אבג def` every Ctrl+arrow press moves the painted caret the way it points and lands at the
  far edge of the word crossed — at the right of the Hebrew run going right, at its left going
  left. Before: the second Ctrl+Right landed at `upstream(7)`, at the run's left.
- `…::ctrl_and_an_arrow_on_a_line_in_one_direction_move_through_the_words`: from every boundary
  of `foo, bar  baz_1 qux.` and of a Hebrew line, both directions, the stop is the logical one.
- `…::ctrl_and_an_arrow_cross_the_lines_of_a_wrapped_field`: forwards through a wrapped Hebrew
  field the stops are the logical word ends on every line, and backwards always moves back.
- `…::a_letter_typed_at_a_change_of_direction_has_the_caret_beside_it`: `x` typed before alef
  paints the caret right after the `x`. Before: at the right of the Hebrew run.
- `…::typing_leaves_the_caret_beside_what_was_typed` (replacing the old test that asserted the
  downstream side): upstream after a letter, downstream after a deletion, a line break, an Enter.
- `frus-test` goldens (WSL, llvmpipe, `CARGO_INCREMENTAL=0`): the new golden, and the whole
  suite.

## Not done

- **A strong letter typed into the other run** still goes in at the offset: with the caret
  drawn after the space in `abc |אבג`, a Hebrew letter is inserted before alef and appears at the
  run's right; the caret then follows it there.
- A single-line field scrolls to keep the **downstream** caret in view, as before; after typing
  at a far boundary of a long mixed line the upstream caret can stand outside the scrolled view.
- The IME paths (composition and its commit) still set the caret downstream.
- A `SelectionArea` has no caret, and its keyboard selection is unchanged.
- Not run on a phone, in a browser, or with a real keyboard on a desktop window.
