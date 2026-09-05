# Milestone 455 — The shapes milestone 451 said it had done

Milestone 451 was called *six widgets that took a corner now take a shape*, and its
CHANGELOG entry read:

> **`shape()` on `Card`, `Chip`, `Dialog` and `Button`, and `shape` on their themes.**

Auditing the crate for the remaining `shape` properties turned up that this was not
entirely true, and that one widget had been left out altogether.

## `Card::shape()` did not exist

The card got the **field**, the four-rung resolution, and a test proving a theme's shape
outranks its radius. It never got the builder. A caller could set the field only through
`radius()`, so the one thing the CHANGELOG announced was the one thing missing.

The lesson is small and worth writing down: a resolution with no way to reach it reads as
finished from the inside. The test that would have caught it is the one that calls the
builder, and there wasn't one — every card test went through `radius`.

## `SnackBar::shape()` did not exist either, and the resolution was misread

Same shape of mistake, plus a second:

```rust
let shape = crate::resolve_shape(
    t.shape,   // <- the *caller's* slot, holding the theme's answer
    None,      // <- the theme's slot, holding nothing
    ...
```

`resolve_shape(own, themed, radius, fallback)` takes the caller's word first. The snack bar
passed the theme's into that slot and nothing into the theme's. The result was *correct* —
with no caller to outrank it, one rung down is the same answer — and it is the kind of
correct that stops being correct the moment the missing argument arrives. It now reads
`resolve_shape(self.shape, t.shape, …)` and there is a `shape()` to fill the first.

## `AppBar` was not one of the six

It had a property **named** `shape` typed `impl Into<BorderRadius>`, with a doc comment
arguing the narrowing:

> The reference's `shape` is a whole `ShapeBorder`; this is the part of it a bar actually
> uses — a rounded rectangle, per corner if wanted.

That argument was true before milestone 450, when there was no `ShapeBorder` to hold. It
expired the day there was, and `AppBarTheme::shape` was then the **last** theme field in
the crate carrying the reference's word with a corner radius behind it — the exact
deviation milestone 451 closed for `DialogTheme`.

### What a shape means on a bar

A bar is a rectangle and stays one. What a shape contributes is the corners it **resolves
to in the bar's own box** (`as_rounded`), which is the same rule milestone 451 used for
shadows.

That is not a compromise, it is the useful part:

```rust
AppBar::new("Inbox").shape(ShapeBorder::stadium())
```

rounds to half the bar's height — 32 on a 64-high bar, 40 on an 80-high one — with the
caller doing no arithmetic. The old radius-only property could not express that at all: the
number would have had to be written at the call site, and would have been wrong for a bar
of any other height. It is the same bug milestone 451 fixed in `Button`, one file over.

A shape with **no** rounded form leaves the bar square rather than clipping it to something
a bar is not.

`radius()` is the shorthand, writing into the same one field — so a caller who passed a
number to `shape` has a one-word change, and this is a **breaking change** for anyone who
did.

## Still not there

Four widgets where the reference has a `shape` and this has nothing at all —
`BottomSheet`, `Menu`, `DatePicker`, `TimePicker` — and one where it has a single `f32`:
`Drawer`. The drawer is the interesting one and is **not** merely mechanical: the reference's
default is `BorderRadiusDirectional.horizontal(end: 16)`, a radius that follows the reading
direction, and this framework has no directional radius. A drawer rounds its **inner** edge,
which is the left or the right depending on the side it is on and on whether the text runs
left to right — a `ShapeBorder` cannot know any of that. Recorded rather than guessed.

## The tests

- `a_bar_takes_a_whole_shape_and_not_only_a_corner` — the stadium at two heights, the
  shorthand, and a bevel leaving the bar square.
- `a_theme_names_the_bar_s_shape`.
- `a_card_takes_a_whole_shape_and_not_only_a_corner` — including both orders of the two
  builders, since they are one field.

Both app bar tests fail with the bar given back the only shape it could hold before — a
rounded rectangle's own literal corners, and nothing from any other shape.

The card and the snack bar have a stronger proof than a failing test: without the builder
this milestone adds, the test **does not compile**. The API was not wrong, it was absent.

**The goldens did not move**: nothing in the pictures asked for a shape it could not have.
