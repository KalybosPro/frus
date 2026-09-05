# Milestone 450 — A box could not say what shape it was

A box here had a `radius: f32` and, at best, a `BorderRadius`. The reference has a whole
family, and `shape` is not decoration — it is the property that decides a chip is a
stadium, a floating button is a circle, and a navigation rail's selection indicator is a
pill rather than a rounded box.

A framework with no way to say *what shape this is* cannot accept the property at all.
That is why `indicatorShape`, `Card.shape`, `Chip.shape` and the rest have sat recorded as
blocked since milestone 437 — not because each was hard, but because the type they all
need did not exist.

## `ShapeBorder`

Four shapes, and a `BorderSide` around any of them:

| | |
|---|---|
| `RoundedRectangle { radius }` | corners rounded by a radius each |
| `Stadium` | a pill: the ends are semicircles, whatever the proportions |
| `Circle { eccentricity }` | a circle in a square out of the middle, or an ellipse on the way to filling the box |
| `Beveled { radius }` | corners cut off straight |

`Copy`, deliberately. This is meant to sit in a theme, and the theme's per-widget structs
are copied wholesale. A shape with a variable number of corners — the reference's
`StarBorder`, `LinearBorder` — would end that, and is not here.

## Two ways out of one type

Three of the four **are** rounded rectangles once the box is known:

- a stadium's radius is half its short side (`stadium_border.dart:95`);
- a circle's box is the one `_adjustRect` takes out of the middle
  (`circle_border.dart:126`), with half *its* short side.

`ShapeBorder::as_rounded` says so, and hands back **the box and the radii** — two things,
because a circle does not occupy the box it was given. Those three go down the renderer's
existing rectangle path and cost nothing new.

`ShapeBorder::outline` answers for all four, as a path. That is what a bevel needs — eight
straight segments where the rounded one has four arcs — and what clipping to a shape will
need when it arrives.

`Scene::draw_shape` picks between them, so no call site has to. A caller doing that by
hand at each site gets the fast path wrong somewhere, which is the whole reason it is one
function.

## Its first user

`NavigationRail`/`BottomBar` destinations take an `indicator_shape`
(`navigation_rail.dart:1148`) — the recorded item this was built to answer, with the theme
answering for all of them underneath.

The old line was:

```rust
scene.draw_rect(pill, fill.fade(o), pill_h * 0.5, 0.0, Color::TRANSPARENT);
```

`pill_h * 0.5` is the arithmetic a stadium does, written out at the call site. It painted
the right thing; it just could not be *told* anything else, and the shape had to be read
back out of a number. The goldens did not move, which is the point: this changes what can
be said, not what is drawn by default.

## The tests

Five on the shape: a stadium is a pill at any size and reads the *short* side either way;
a circle takes a square out of the middle and walks out to the box as eccentricity rises;
a bevel is the one that needs a path, and every shape still answers with an outline; a
corner is capped by its box; an edge is drawn only when there is one.

Two on the destination — a pill by default, a caller's own shape over it, the theme
between them. Both fail with the inferred radius restored.

## Still open

`ContinuousRectangleBorder` — the reference's fifth — is a superellipse, not a shape that
falls out of arcs, and it is the one Material uses for its "squircle" corners. Recorded.

And the shapes are in place before the widgets that want them: `Card`, `Chip`, `Dialog`,
`Button`, the FAB and the snack bar all take a `shape` in the reference and take a radius
here. Each is now a small change rather than a blocked one.
