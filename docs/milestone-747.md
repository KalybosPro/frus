# Milestone 747 — A caret on the right side of a change of direction

## Objective

Milestone 742 made a line that mixes directions highlighted correctly, one box per stretch on
screen, and left in its "Not done": the caret at the boundary between two directions stood at
only one of the two places it could, a press there picked the nearer of the caret stops the
layout knew, a field's bidirectional editing had never been looked at, and a soft-wrapped
right-to-left paragraph had been tested on one line only.

A boundary between two characters is usually one place on screen. Two kinds are two places:

- where a line **wraps** without a newline, the boundary is the end of one line and the start
  of the next;
- where the **direction changes** — `abc אבג def`, or digits inside Hebrew or Arabic — the
  boundary between the runs is drawn at the edge of each run, often a whole word apart.

A caret that knows only its index cannot say which of the two it is. It was always drawn at the
second (the leading edge of the character after it), so the caret jumped across a run as it was
moved, a press beside a run put it a letter or a word away from the finger, and End on a wrapped
line drew it at the start of the next line.

## What was done

- **A caret position carries an affinity.** `frus-core` gained `TextAffinity` (`Upstream`,
  `Downstream`, the default) and `TextPosition { offset, affinity }`, re-exported by
  `frus-widgets` and `frus`. Downstream belongs to the character after the boundary — the caret
  stands at its leading edge, on its line; upstream belongs to the character before — at its
  trailing edge, on its line. `Edit` (a field's caret and selection) and `Status` (what a field
  paints from) gained an `affinity` field.
- **`TextLayout` answers in positions.** `caret_rect_at(position)` draws a caret at either side;
  `caret_rect(index)` is the downstream one, as before. `position_at(point)` resolves a press by
  the character under it and the half that was pressed: its leading half is the boundary before
  it, downstream; its trailing half the boundary after it, upstream. Beside a line, or past its
  end, it is the nearest edge — and of two at one place the later in the text, so a press past
  the end of a wrapped line is that line's end, after the space it broke at. `hit_test` is the
  offset of `position_at`. Every line now records the direction each of its characters was drawn
  in and the direction of the line itself.
- **The arrows move the caret the way they point.** `TextLayout::visual_step(from, rightwards)`
  crosses the character drawn next to the caret on that side, whatever its direction, and leaves
  the caret beside it: rightwards over a left-to-right letter lands after it (upstream),
  rightwards over a right-to-left letter before it (downstream). Off the end of a line — its
  right for a left-to-right line, its left for a right-to-left one — the caret goes to the start
  of the next line; off its start, to the end of the line before. Ctrl and an arrow still moves a
  word at a time through the text's order, forwards being leftwards on a right-to-left line.
- **Home and End go to the ends of the line on screen.** `TextLayout::line_bounds(position)`
  gives the line's first boundary downstream and its last upstream, so End on a wrapped line
  puts the caret at the end of that line — after the space it broke at, drawn on it — and Home
  brings it back. Ctrl+Home and Ctrl+End are still the ends of the whole text. Before, Home and
  End went to the ends of the paragraph between two newlines, which in a wrapped field was
  several lines away.
- **Up and down keep the side.** `Widget::caret_vertical` takes and returns a `TextPosition`:
  down from the end of a wrapped line starts from where that end is drawn, not from the start of
  the next line, and lands on whatever side of the boundary is below the column.
- **Fields and selectable texts.** A new `Widget::caret_key(width, edit, key)` moves the caret
  for an arrow, Home or End over the text as it is drawn in a box `width` wide; the shell asks it
  first and gives the key to `on_edit` only when a widget has no drawn text to move over (which
  then moves through the text's order, as before). `TextField`, a selectable `Text` and a
  selectable `RichText` answer it from the same layout they paint and hit with — alignment,
  wrapping and cuts included. `Widget::cursor_at` returns a `TextPosition`, and the shell keeps
  its affinity for a click, a drag, a right click and a hold. A field paints its caret on the side
  it belongs to. Typing or deleting puts the caret downstream again, before what follows it.
- **Selection handles stand at the edges of the selected characters**: the start handle at the
  leading edge of the first, the end handle at the trailing edge of the last. A selection that
  ends at a wrap has its end handle on the line it ends on, not hanging from the start of the
  next.
- The forwarding wrappers (`Box<dyn Widget>`, the transparent wrappers, `Responsive`) forward
  `caret_key` and the new signatures.
- **A field's double click is two presses close together.** The shell told a double click in a
  field by the wall clock alone: any second press within 0.4 s, anywhere in the field, selected
  the word under it. Two quick clicks at opposite ends of a line selected a word instead of
  placing the caret twice, and in a test the driver's presses — microseconds apart on the wall —
  made every second click a double click. The second press must now land in the same field,
  within 8 logical pixels of the first for a mouse (100 for a finger, the double tap's slop), and
  the time is counted on the clock the gestures have used since milestone 584: the frames run in
  a driven test, the wall in an application. A selection area's double click, which already
  had the distance, is counted on the same clock with the same rule.

## Decisions

**Affinity, not two carets.** Some editors draw a split caret at a direction boundary — a tall
half at one place, a short one at the other. A caret that belongs to one side and is drawn there
is what the leading cross-platform toolkits do, what a phone shows, and what an accessibility
service can describe; and it needs one bit, not a second rectangle, so it rides in `Edit` and
`Status` at no cost.

**Visual arrows, logical selection.** The arrows move on screen in every major browser, on
Android and on macOS; a caret that moved rightwards through Latin and then leftwards through a
Hebrew word because that is the text's order reads as broken to someone writing both. The
selection itself stays a range of the text's order — that is what Copy, deletion and typing work
on — so Shift and an arrow across a change of direction extends the selection by the character
crossed in logical order, and the highlight, which milestone 742 draws per stretch, can grow on
the far side of the run. This is how those platforms behave too.

**Least disruptive shape.** `Edit` keeps `cursor: usize` and gains `affinity` beside it rather
than becoming a pair of positions: every place that reads or computes an index — undo, the IME,
the web input bridge, the selection area — stays as it was, and the affinity defaults to
downstream, which is the old behaviour. The caret keys moved onto a new hook with the width
because `on_edit` has none and a wrapped line's ends cannot be found without it; `on_edit` keeps
handling them, through the text's order, for any widget that does not answer the hook, and its
direct tests are unchanged. The three hooks that took or returned a bare caret index
(`cursor_at`, `caret_vertical`, `text_metrics`) take and return a `TextPosition` instead — a
breaking change for a widget that overrides them outside frus, noted in the changelog.

**Where a crossed character leaves the caret.** Beside it, belonging to it. So after Right over
the last letter of a wrapped line the caret is at the end of that line (upstream), and the next
Right takes it to the start of the next line (downstream) — two presses for two places, the same
boundary. Without it the caret would vanish from the end of the line it was on.

## Tests

Tests were written first; they named an API that did not exist, and the behaviour they pin did
not exist either — a press on the left of the last Hebrew letter of `abc אבג def` landed on the
boundary before the space (the nearest downstream stop), a field's End went to the end of the
paragraph, and a selection's end handle at a wrap hung from the next line.

- `frus-text`: `abc אבג def` has a caret on each side of the boundary before the Hebrew run —
  after the space, and at the right of alef — and of the boundary after it; a press on either
  side of a change of direction gives the side pressed, and wherever the press, the caret is at
  most half a character from it; the arrows walk the line rightwards with every step to the
  right, every character crossed once, and back; in Hebrew with `123` inside, the boundary before
  the number is drawn at either end of the number and the arrows cross 3, 2, 1 in that order; a
  Hebrew paragraph wrapped on several lines starts every line at the right, has the wrap
  upstream at the left end of the line above and downstream at the right of the line below, takes
  a press at the far left of the first line as its end, on it, bounds its lines for Home and End,
  and is walked leftwards line by line; an English paragraph's wrap is the mirror of it.
- `frus-widgets` (`textinput.rs`): a field's painted caret moves right at every Right press
  through `abc אבג def` and back with Left; a click on the left of the last Hebrew letter paints
  the caret there; End in a wrapped Hebrew field paints the caret at the left end of the first
  line, Home brings it to the start, Shift+End selects the line and its end handle hangs from that
  line; Down from the end of the first line lands about below it on the second and Up comes back;
  Shift+Left extends a Hebrew selection a letter at a time; typing puts the caret downstream.
- `frus-shell`, the double click: two quick presses far apart in a field select no word, two
  close together select the word, and a third a second later is a single click again. With the
  old rule put back (the wall clock, no distance) the first assertion fails: the far pair
  selected `delta`.
- `frus-shell`, through the driver with a mouse and keys: a click beside the change of direction
  in a field paints the caret where the pointer went down, Right takes it to the end of the line
  and Left back to the start, never the wrong way; in a wrapped Hebrew field End stays on the
  first line at its left end, Home comes back to its right, and Down from the end reaches the
  second line.
- Every existing test of the plain layout, of fields, of selectable texts and of selection areas
  passes unchanged in what it checks (call sites adapted to the new signatures).

## Not done

- **Ctrl and an arrow** moves a word at a time through the text's order (reversed on a
  right-to-left line), not visually by word through a mixed line.
- **Typing at a direction boundary** inserts at the offset whatever the affinity: a Hebrew letter
  typed with the caret drawn after the space in `abc |אבג` lands logically before alef, which is
  drawn at the run's right. Choosing the insertion point by the caret's side is not done.
- A single-line field scrolls to keep the **downstream** caret in view, as before; an upstream
  caret at a far boundary of a long line can stand outside the scrolled view.
- A `SelectionArea` has no caret, and its drag ends are resolved by offset only.
- Not run on a phone, in a browser, or with a real keyboard on a desktop window; the IME paths
  set the caret downstream.
