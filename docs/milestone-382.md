# Milestone 382 — A wrap where the lines are a decision too

`Wrap` had one spacing.

```rust
Wrap::new().gap(8.0).child(chip_a).child(chip_b) // …
```

Eight pixels between the chips on a line, and eight between the lines. There was no way to
ask for anything else, because `Flex` had one `gap` and wrote it into both axes.

That is the wrong shape often enough to be worth two numbers. A wrap of chips usually wants
them close side by side and further apart line to line: the eye reads a line as a unit, and
the vertical break is what tells it where one line ends and the next begins. With a single
gap you pick whichever compromise is least bad.

`run_gap` is the second number — the reference's `runSpacing` to `gap`'s `spacing`. Untold,
the lines are still spaced by `gap`, so nothing that says nothing changes.

## Which axis the runs stack on is not a constant

`frus-layout::Style` already had `row_gap` and `column_gap` — the grid uses them — so the
obvious move was to write `run_gap` into `row_gap` and stop.

That would have been wrong for half the framework's wraps. The lines of a wrapping **row**
stack downwards, so the spacing between them is the row gap. The lines of a wrapping
**column** stack sideways, so it is the column gap. `Flex::column().wrap()` is a legal and
useful container, and it would have silently ignored `run_gap` entirely.

Worse than silently ignoring it: it would have worked perfectly on every wrapping row
anybody happened to test.

```rust
row_gap: match self.direction {
    FlexDirection::Row | FlexDirection::RowReverse => self.run_gap,
    _ => None,
},
column_gap: match self.direction {
    FlexDirection::Column | FlexDirection::ColumnReverse => self.run_gap,
    _ => None,
},
```

## The lines had no alignment at all

`align` places each child within its line. Nothing placed the **lines**. `Style` had no
`align_content`, so a wrapping container with cross-axis room to spare always got flexbox's
default — the lines stretch to fill it — and no caller could say otherwise.

`AlignContent` is the new enum, `Flex::align_lines` the builder, and `Stretch` stays the
default so no existing layout moves.

## The test was wrong before the feature was

The first draft of `a_wraps_lines_can_be_packed` compared a stretched wrap against a packed
one and got `y = 0` from both. The feature looked broken.

It was not: a `Wrap` sizes to its content, so there was no cross-axis room to distribute and
the lines sat where they sat, whatever they were told. The container was wrong, not the
alignment. Giving the wrap a height made both cases behave, and the comment in the test now
says so, because the next person to write a test against `align_content` will make the same
assumption.

The other tests measure the **laid-out rectangles** rather than reading the field back off
the style, for the same reason: a field set and never reaching the layout engine is exactly
the failure worth catching, and a test that asks the style what the style says will never
catch it.

## Left

The reference's `Wrap` also has `crossAxisAlignment` per run — how a child sits within its
own line when the lines differ in thickness — and `verticalDirection`, which is a
bottom-to-top flow. Neither is reachable through `Align` today, and each is its own step.
