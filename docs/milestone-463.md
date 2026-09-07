# Milestone 463 — An expansion tile could not be themed, and had no shape

`ExpansionTile` already had a good set of builders: `background_color`,
`collapsed_background_color`, `text_color`, `collapsed_text_color`, `icon_color`,
`collapsed_icon_color`, `tile_padding`, `children_padding`. Nine properties, named after
the reference's, resolved in the right order.

What it had nowhere was a place to say any of them **once**.

That is the exact gap `ListTile` had until milestone 458, on the widget usually built
*out of* list tiles — a settings screen is a column of expansion tiles, so an application
that wanted its open sections tinted said so on each of them, and a tenth section written
next quarter says it again or does not match.

`ExpansionTileTheme` has ten entries. Every colour comes in a pair, because an expansion
tile is two things: the row that is always there, and the row while what it hides is
showing.

## The shape it did not have

`ExpansionTile` had no `shape` at all, where the reference's theme has two — `shape` and
`collapsedShape` (`expansion_tile_theme.dart:54`). It could take one now for free: the
row is a `ListTile`, and a list tile has taken a shape since milestone 457, on its
surface **and** its ink.

The two stay apart. A tile that names only the open shape does **not** get it when shut —
which looks like an omission and is a decision: square-when-shut, rounded-when-open is a
design, and a fallback would make it unsayable. That is the same reasoning as the
drawer's `end_shape` in milestone 456, and the test that holds it is named after the
mistake it prevents.

`radius(n)` is the shorthand for both, which is what an application usually means.

## The field that could not say no

```rust
children_padding: Insets,   // = Insets::new(0.0, 16.0, 16.0, 16.0)
```

A bare `Insets` with a value baked into the constructor. So *nothing was said* and *zero
on every side* are the same value, and:

- a theme rung under it could never fire, because it would always find an answer already
  there;
- and `children_padding(Insets::ZERO)` was indistinguishable from a tile that had never
  mentioned it.

It is now `Option<Insets>` with the number moved out to a named `CHILDREN_PADDING`. This
is exactly the trap `AccessibilityOverrides` was built to avoid in milestone 407 — a
`false` in a plain struct being indistinguishable from silence — and it is worth noticing
that it turned up again in a widget three hundred milestones later. **A default written
into a constructor is a decision that cannot be reversed by anything downstream of it.**

## The tests

Three, all of which fail when the milestone is undone — checked by putting the fallback
back on `collapsed_shape`, dropping the surface rung, and hard-coding the body's room.

`a_theme_answers_for_every_expansion_tile` reads the row's **painted** surface and
corners out of a built frame, rather than asking the tile what it thinks: the tile hands
its answers to a `ListTile`, so the only assertion worth making is about what came out
the far end.
