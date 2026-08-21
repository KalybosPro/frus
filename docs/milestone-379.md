# Milestone 379 — A drawer nobody could recolour

`DrawerTheme` had exactly one field.

```rust
pub struct DrawerTheme {
    pub width: Option<f32>,
}
```

Everything else the panel painted was written into `paint` and stayed there: the fill was
`theme.surface`, the hairline was `outline_variant`, the hairline was 1 px, the corners were
square, and the scrim behind a modal one was the `scrim` role at a fixed half opacity,
painted centrally in the walk where no drawer could reach it.

The same breach milestone 368 found on `ExpansionTile` and milestone 378 found on `Badge`:
themed defaults are fine, hardcoded-only never. This is the third widget down that list and
the first one where the hardcoding was hiding a bug.

## Which edge is the inner one is not known until it paints

The panel drew its hairline on the side it was *asked* for:

```rust
let x = if self.border_left { bounds.x } else { bounds.x + bounds.width - 1.0 };
```

`border_left` was set from `Drawer::right` — the **logical** side, leading or trailing. But
a leading drawer is on the left of the screen in English and on the **right** in Arabic: the
walk mirrors the whole frame, and the modal placement flips with it. So in RTL the panel
landed against the right edge of the window and ruled its hairline down that same right
edge — a line along the window's own border, with the edge that actually faces the content
left bare.

It is now `end: bool` — the logical side, named as such — and the screen side is worked out
in `paint`, where the direction is in the theme that is in force:

```rust
fn docked_right(&self, theme: &Theme) -> bool {
    self.end != (theme.direction == TextDirection::Rtl)
}
```

The rounding needs exactly the same answer, which is why the bug was worth finding before
the shape went in rather than after.

## The shape

The reference's drawer rounds its **inner** edge by 16 px and leaves the outer one square —
the corners that face the content, not the ones that face nothing. Its own source calls the
figure *shown in the spec* and hardcodes it for want of a token; ours is `DRAWER_RADIUS`,
and it is a default like every other one here.

`BorderRadius` could not express it. There was `top` and `bottom` — the two horizontal
edges — and nothing for the vertical pair, which is the shape a side panel wants. `left` and
`right` complete the set.

The hairline is drawn as its own sliver rather than as a border round the whole shape, for
two reasons that are really one: the other three edges are flush against the window, so a
rule down them would be a line against nothing, and a straight sliver crossing a rounded
corner sticks out past the shape it is meant to edge. It stops short of the corners by the
radius.

## A shadow that falls sideways

`elevation` casts along the **inner** edge, not downwards. A drawer is as tall as the
window; a shadow dropped below it falls outside the window and is never seen. The default
is `0.0` — no shadow — which is where the reference's own M3 defaults land too, by a
different route (`elevation: 1.0` with a transparent `shadowColor`).

## The scrim was nobody's

It was painted in `process_overlays` from the scheme's `scrim` role at `0.5 * progress`, and
that is where every modal in the framework gets its scrim — a dialog, a sheet, a drawer.
Nothing could say otherwise.

`overlay_scrim(&theme)` is the hook. It takes the theme rather than nothing at all, because
a drawer's untold scrim is a `DrawerTheme` entry and a hook with no theme could only ever
read the instance. The alpha is the caller's: a scrim is the one colour whose *transparency*
is the point, so an opaque value hides the body entirely and `Color::TRANSPARENT` is an
overlay that darkens nothing — which the reference reaches by the same route. The progress
still multiplies it, so the fade stays synchronised with the slide.

## `body()` stopped finalising the drawer

Writing the tests turned up a second one. `body()` wrapped the panel there and then, which
made every builder called after it a **silent no-op** — the field it set was never read
again:

```rust
Drawer::new(open)
    .panel(nav)
    .body(screen)
    .width(96.0)          // did nothing
    .background_color(c)  // did nothing
```

Nothing warned. `width` had carried this since it was added; the six builders this
milestone adds would each have inherited it.

The two pieces the caller hands over now sit in `Cell`s and the tree is assembled once, the
first time the walk asks for the children. Order stops mattering, which is what every other
widget in the catalogue already promised.

## One golden moved

`drawer_open.png` — the panel's inner edge is rounded now, and the hairline
stops short of the two corners. Read before it was accepted.

## Left

`semanticLabel`: the reference announces the drawer as a named route, and `Role` has no
region or dialog to announce it as. Adding one is a change to the semantics vocabulary and
its platform mapping, not a builder. `surfaceTintColor` likewise has no counterpart here
yet. And the reference opens a drawer by **dragging** from the screen edge
(`enableOpenDragGesture`, `edgeDragWidth`); ours opens only when the application says so.
