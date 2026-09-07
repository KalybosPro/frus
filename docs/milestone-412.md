# Milestone 412 — Twenty-four places that recomputed a line's height

Milestone 409 gave a style a `height` and threaded it through the measurement, the
one-line box floor and the paint. It was not finished, and the gap is mine: **twenty-four
places went on computing `frus_text::line_height(style.size)` — the 1.2 default — while
holding a style that said otherwise.**

They are almost all vertical centring:

```rust
let ty = bounds.y + (bounds.height - frus_text::line_height(style.size)) * 0.5;
```

A text with `height: 2.0` was measured tall, painted tall, and **centred as though it were
short**. Three answers, two of them right, and nothing to say which.

## The shape, again

This is the fourth milestone in a row on the same shape: two ways to obtain one number, and
one of them not knowing what the other knew. Two doors named `text` (406), two guards (408),
two `LINE_HEIGHT_FACTOR` constants (409), and now a method on the style beside a free
function on its size.

`style.line_height()` is the answer and it already existed — milestone 409 put it there.
These sites simply never asked.

## What changed, and what deliberately did not

**Twenty-four sites** across fourteen files now ask the style. The ones left take a bare
number — `line_height(SIZE)`, `line_height(FIELD_TEXT_SIZE)` — where there is no style whose
height could be honoured. Those belong to a different problem, below.

Two mistakes on the way, both caught by the compiler rather than by reading:

- The first sweep matched only named styles (`style.size`), not styles produced by a call
  (`label_style().size`). Four more sites at the second pass.
- The second sweep was **too** wide: it turned `line_height(inp.text_style().size)` into
  `inp.line_height()`, calling the method on the *widget*. This is exactly why the
  formulation to delete is the one the compiler can find.

## The test, and the ones that are not possible yet

`a_limit_counts_lines_of_the_height_that_was_asked_for`: a `max_lines(2)` cap is
`line_height × 2`, so a text with a doubled leading must occupy twice the box of the same
text at the default. Before the fix the cap was computed at 1.2 whatever the style said, and
the ratio would have been 1.

That is one of the twenty-four. **The other twenty-three are not reachable by a test**,
because they live in widgets — `Menu`, `Dropdown`, `DatePicker`, `Toast`, `Tree` — whose
text style is a private constant no caller can change. A style that cannot carry a `height`
cannot demonstrate that its `height` is honoured.

## The problem this uncovered

Twelve widgets decide their own type in a private constant, with no theme and no override:
`Kbd`, `Toast`, `Tree`, `Timeline`, `Steps`, `NavRail`, `DatePicker`, `Alert`, `Menu`,
`Dropdown`, `Autocomplete`, `Table`. That is the standing rule taken backwards — themed
defaults yes, hardcoded-only never — and it is why most of this milestone is untestable.

It is worse than unthemed. The reference's Material 3 snackbar content is `bodyMedium`,
**14 px**; our `Toast` says `const SIZE: f32 = 16.0`. The constant had drifted two pixels
from the reference and **nobody could see it, because it was private**. The action is 14 px
regular where the reference's is `labelLarge`, 14 px medium.

Recorded as the next step rather than folded in here: it moves goldens, and a milestone that
mixes a mechanism with a change of appearance makes both harder to review.

## Left

- The twelve hardcoded widgets, above.
- `richtext.rs:269` and the `textinput.rs` sites still derive from a bare size, for the same
  reason.
