# Milestone 456 — A radius that follows the reading direction

Milestone 455 ended by recording why the drawer could not be given a shape like the other
widgets, and that it was **not** merely mechanical:

> The reference's default is `BorderRadiusDirectional.horizontal(end: 16)`, a radius that
> follows the reading direction, and this framework has no directional radius.

This is that type, and the drawer that needed it.

## The question a `BorderRadius` cannot ask

A `BorderRadius` says *top left*. That is a statement about the **wall**, and an interface
that mirrors has a different question: which corner is at the **beginning of the line**?

For most things the two coincide and nobody notices. For anything asymmetric they part
company, and a drawer is the clearest case in the crate: a panel rounds its **inner** edge,
the one facing the page. For a leading panel that is the *end* side — the right in English,
the left in Arabic. Written as `BorderRadius::right`, it is correct in English and wrong in
Arabic.

```rust
BorderRadiusDirectional::end(16.0).resolve(TextDirection::Ltr) == BorderRadius::right(16.0)
BorderRadiusDirectional::end(16.0).resolve(TextDirection::Rtl) == BorderRadius::left(16.0)
```

`top_start`, `top_end`, `bottom_end`, `bottom_start`, with `uniform`, `start`, `end`,
`horizontal` and `vertical`, and one `resolve`.

The reference models this as a type hierarchy — `BorderRadiusGeometry`, with a resolved and
an unresolved subclass, so an API can accept either. This is two plain types and a
`resolve`, because a widget here is handed a `&Theme` and therefore always has a direction
in hand at the moment it needs one. There is nowhere the ambiguity has to survive.

## The drawer said it twice

The panel had this:

```rust
match self.docked_right(theme) {
    true => BorderRadius::left(r),
    false => BorderRadius::right(r),
}
```

which is *correct* — `docked_right` folds together which side the drawer is on and which
way the text runs, and the answer comes out right in both directions. It is correct
arithmetic standing in for a word the framework did not have, exactly like the button's
`height / 2` in milestone 451.

It now says what the reference says (`drawer.dart:801`, `:810`): a leading panel rounds its
**end** corners, a trailing one its **start** corners, and `resolve` decides which wall
that is. Same pixels; one fewer thing worked out by hand.

## And it takes a shape

`Drawer::shape()` over the `radius()` shorthand, on the usual four rungs: the caller, the
theme, the theme's plain radius **on the inner edge**, then the framework's own.

`DrawerTheme` gains `shape` **and** `end_shape` — two fields, as the reference has
(`drawer.dart:268`, `:269`), not one mirrored. A trailing panel does not fall back to the
leading panel's shape, and that is deliberate: a theme that named only one has said nothing
about the other, and the framework's default is a better answer than a shape rounded on the
wrong edge. The reference reaches the same conclusion with the same two lines.

A caller's shape is **concrete** — it says left and right — so naming one takes the
mirroring on. `BorderRadiusDirectional` is how a caller answers it, and the doc comment
points at the framework's own default as the worked example.

## The tests

- `a_directional_radius_follows_the_reading_direction` — every constructor across the
  mirror, including the two that have no side and must not move.
- `a_panel_takes_a_whole_shape` — a caller's shape, a theme's on the panel it names, and
  the trailing panel falling to the framework rather than to the leading panel's shape.

**The goldens did not move**, which is the point: the drawer's pixels are what they were.
