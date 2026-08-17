# Milestone 333 — Half a fix, and the other half named

The device finding from milestone 327: a long task label pushes the row's delete button out
of the card. Reproducing it through `Application::view` made it worse than reported — the
long row has **no delete target at all**, while the short row beside it has one at
`x = 206`. The button is not merely clipped; it is off the card, off the window, and out of
the hit registry. That task cannot be deleted with the ×.

## Where it is not

The row is width-constrained: both rows measure `x = 44, w = 336`. So the card does its
job, and this is not a missing constraint.

Nor is flexbox shrinking broken. Isolated — a 200 px row holding a 558 px label and a 40 px
button — the row resolves to exactly 200. Taffy shrinks.

## Where it is

It shrinks the **wrong child**. The deficit is 398 px and flexbox shares it in proportion to
base size, so:

| item | base | shrunk to |
|---|---|---|
| label | 558 | (refuses) |
| button | 40 | **13** |

The label refuses because a flex item's automatic minimum size is its own content — the
classic `min-width: auto` floor, which the web has needed `min-width: 0` for since
flexbox shipped. So the whole deficit lands on the button.

Two things are needed, and only one of them existed to be fixed.

## What is fixed: `Text::ellipsis()`

The reference's `TextOverflow.ellipsis`, and `Text` had no equivalent — no ellipsis, no
`maxLines`, nothing but `wrap()`, which grows downwards. The capability *did* exist:
`AppBar` carried a private `truncated()` for its own title. One implementation, in the one
place that is not the text widget.

`Text::ellipsis()` is now the public form of it, and `truncate` moved to `text.rs` where
both callers can see it. It does two things:

- **cuts** the line to whatever width it is given, ending in an ellipsis;
- **accepts** less width than it asked for, via `min_width: 0`.

The second is the one that matters and the one nobody would think to ask for.

## What is not fixed, precisely

The button's half needs the opposite: *do not shrink me*. That is `flex-shrink: 0`, and
`frus_layout::Style` has no `flex_shrink` field at all — not on `Flex`, not on `Container`,
nowhere. Taffy underneath defaults it to `1.0`, which is why everything shares the deficit
and nothing can opt out.

So this milestone stops here rather than pretending. Adding the field, plumbing it, and
exposing it on the widgets that need it is a layout change with its own goldens and its own
device check, and the demo's row should not be edited until the framework can express what
it needs. The roadmap carries the measurement.

## A wart, pinned

`truncate` returns the content untouched when `max_width <= 0.0`. That is inherited from
the app bar, where a zero width means *the layout has not run yet* rather than *there is no
room* — but it means a genuinely collapsed box draws its whole string, which is exactly the
overflow ellipsising exists to prevent. Told apart it should be; guessed at it should not.
There is a test asserting the current behaviour so that changing it is a decision.

## Left

- **`flex_shrink`**, above.
- **`maxLines`.** The reference's `Text` cuts at a line count as well as a width. Ours is
  one line or a paragraph, with nothing in between.
- **The demo's row is untouched**, so the device finding is still live. It needs both
  halves and a device to confirm, because what the report was really about is the hit
  registry, not the picture.
