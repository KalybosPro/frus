# Milestone 404 — "Full width", said the one way that cannot work

Milestone 403's scaling probe printed something that had nothing to do with scaling:

```
bare:                    "A row"  box 46x20
in a column:             "A row…" box  0x20
in a stretched column:   "A row…" box  0x20
```

A `ListTile` **on its own** lays out correctly. Put it in a column — the most ordinary
thing anybody does with a list tile — and its title box collapses to zero width and the
text ellipsises away.

## The cause, in one line of code

```rust
width: Dimension::Percent(1.0),
```

A percentage width resolves against the parent's **resolved** width. A parent that
shrink-wraps has not got one yet: it is waiting on this very child to find out how wide it
should be. So the percentage resolves against nothing, and the box comes out empty.

Both readings are "full width" in English, and only one of them can be computed in time:

| | known | when |
|---|---|---|
| `width: 100%` | the parent's own width | on the way back **up** |
| a fill request | the room being offered | on the way **down** |

The framework already has the second — `Widget::main_axis_fill`, answered by the walk,
which is exactly where both the parent and what it was offered are in view. `SafeArea`
needed the same word in milestone 393 and got it.

The mechanism even defends itself. `Fills::bounded_by` drops a fill request on an axis the
box was **given a size on**: "it has already been told how big to be, and a request to fill
cannot reach past that answer." So a widget that declared `Percent(1.0)` could not have
asked, even if it had tried.

## It was not a `ListTile` bug

Fifteen widgets declare `width: Percent(1.0)`. A probe built seven of them alone and in a
column:

```
ListTile:     alone 400  |  in a column 40      (its own padding)
BottomAppBar: alone 400  |  in a column 16      (its own padding)
BottomSheet:  alone 400  |  in a column 0
Drawer:       alone 400  |  in a column 0
TwoPane:      alone 400  |  in a column 0
AspectRatio:  alone 400  |  in a column 0
Steps:        alone 400  |  in a column 0
```

Every one. This is a framework-wide idiom bug, not one widget's mistake.

**Why no test ever caught it**: a percentage against a *definite* parent is correct, and
every fixture in the suite gives its widgets a width. Alone is where the bug hides.

## What changed, and what deliberately did not

**Converted** — nine impls across eight files: `ListTile`, `BottomAppBar`, `SheetPanel`,
`BottomSheet`, `Drawer`, `Steps`, `BarChart`, `LineChart`, `Bullet`, `ErrorSummary`. Each
drops the declared width and answers `main_axis_fill` with `Row`. Their heights are
untouched: those are fixed or content-derived and were never the problem.

**Kept, with the reason strengthened rather than removed:**

- `AspectRatio` needs a width taffy **knows** in order to derive the height from the ratio.
  A stretched width is not enough for taffy to compute the other axis, which is what its
  comment already said. So it goes on collapsing in a shrink-wrapping parent, and that is
  now written down instead of being a surprise.
- `ConstrainedBox`'s overflow case, for the same reason: its child is laid out separately
  and contributes nothing, so the box must be told a size rather than derive one.

**Recorded, not fixed**: `NavScaffold`, `ScaffoldMessenger` and `TwoPane` want to fill
**both** axes, and `main_axis_fill` returns one. They are root-level shells that in practice
sit under a definite parent — but "in practice" is precisely the reasoning that has been
wrong three times in this codebase, so it is a roadmap entry rather than a shrug. The hook
needs to be able to say *both*, and that is a public API change worth its own step.

## The test

`widgets_that_fill_the_width_do_it_in_a_column_too` checks five widgets **alone and in a
column**, asserting each is the full 400 it was offered. The pairing is the point: alone
already passed before the fix.

## Left

- `main_axis_fill` cannot ask for both axes (above).
- The remaining `Percent(1.0)` heights have the same shape of problem against a
  shrink-wrapping parent on the vertical axis. Nothing has reported one, and this milestone
  did not go looking — the horizontal case is what a probe found and what is fixed.
