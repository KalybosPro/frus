# Milestone 405 — A widget can ask for both axes

Milestone 404 converted nine widgets from `width: 100%` to a fill request, and had to leave
three behind: `NavScaffold`, `ScaffoldMessenger` and `TwoPane` want the room they are
offered on **both** axes, and the hook could only name one.

```rust
fn main_axis_fill(&self, theme: &Theme) -> Option<FlexDirection>
```

One direction. A widget wanting both had no word for it, so those three kept the percentage
that makes them vanish under a shrink-wrapping parent — the very bug 404 was about.

They are root-level shells that in practice sit under a definite parent. "In practice" is
exactly the reasoning that has been wrong three times in this codebase, which is why this
is a milestone rather than a comment.

## The type

`FillAxes` is a flag per axis, with the four answers named:

```rust
FillAxes::NONE     // shrink-wrap on both
FillAxes::WIDTH    // take the width offered, hug the content vertically
FillAxes::HEIGHT
FillAxes::BOTH     // a full-screen shell
```

`fn fill_axes(&self, theme: &Theme) -> FillAxes` replaces `main_axis_fill`. BREAKING for
anyone implementing `Widget` by hand, and mechanical: `Some(Row)` becomes
`FillAxes::WIDTH`, `None` becomes `FillAxes::NONE`.

### Why it is not the walk's `Fills`

The walk already has a two-flag type, and its own doc explains why reusing it would be
wrong:

> It is not a property of one widget. A column whose row fills the width is itself as wide
> as the room it was given, because the row inside it took that room — so the request
> travels up as the layout is built.

`Fills` is an **accumulator**; `FillAxes` is one widget's **answer**. They have the same
shape and different meanings, and `Fills` carries `merge`, `across` and `bounded_by`, which
are the walk's business and no part of a public contract. Keeping them apart cost one
`From`-ish conversion that is now three lines and used to be a nine-line `match`.

## Where the explanation lives now

`FillAxes`'s own documentation carries the table, because this is the third milestone in a
row to run into it and the next person meets the type before they meet the milestone notes:

| | what it needs | known |
|---|---|---|
| `width: 100%` | the parent's own width | on the way back **up** |
| a fill request | the room being offered | on the way **down** |

A doc comment that says "fifteen widgets said it the first way and every one collapsed" is
worth more than a roadmap entry nobody reads at the moment of writing the bug.

## The test

`widgets_that_fill_the_width_do_it_in_a_column_too` grew from five widgets to seven —
`NavScaffold` and `TwoPane` join it, and would have failed on the previous commit.

## Left

- **The vertical axis is untested.** `FillAxes::BOTH` is asserted on the width only,
  because the fixture measures widths. A shell that fills vertically and a column that does
  not have never been checked against each other.
- `AspectRatio` and `ConstrainedBox`'s overflow case still declare percentages, for the
  reason milestone 404 recorded: taffy needs a width it **knows** in order to derive the
  other axis from a ratio, and a filled width is not one.
