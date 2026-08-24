# Milestone 395 — The bar has a surface, and Material 3 tints it

The reference's `AppBar` takes twenty-eight parameters. Ours took eighteen. This milestone
is the group that decides what the bar **looks like as a surface** — the ones a design
system reaches for first — plus the mechanism underneath one of them, which belongs to the
whole framework rather than to the bar.

## The tint, and where it belongs

Material 3 does not show height with a shadow. It shows it with a **tint**: the surface
moves towards a tint colour in proportion to its elevation. That matters most exactly where
a shadow fails — on a dark background, where a shadow shows nothing at all.

The strength is not a curve. It is a table, and the reference generates it from the
specification's tokens:

| elevation | 0 | 1 | 3 | 6 | 8 | 12 |
|---|---|---|---|---|---|---|
| opacity | 0 | 0.05 | 0.08 | 0.11 | 0.12 | 0.14 |

— `elevation_overlay.dart:171`. Between two levels it interpolates; outside them it clamps,
so a bar at 40 is tinted exactly as much as one at 12.

`Card`, `Drawer` and `BottomAppBar` all want this too, so it went into **`frus-core`** as
`Color::surface_tint` and `surface_tint_opacity`, not into the app bar.

**On the colour space**, because this framework's recurring bug is exactly there: the blend
is a plain channel mix and that is *correct here*. The result is an opaque colour computed
once and handed to the renderer to paint — nothing is composited, so there is no linear-space
step to get wrong. Laying the tint on as a translucent layer instead would go through
compositing and come out darker. `Color.alphaBlend(tint.withOpacity(o), base)` and
`base.lerp(tint, o)` are the same arithmetic when `base` is opaque, which is why the test
asserts the surface is the background moved **exactly** 8% towards the tint at elevation 3,
rather than merely that it moved.

## The rest of the surface

- **`shape`** — how far the corners are rounded. It **clips** as well as rounds: a surface
  that stopped short of its own corner would square off the one the shadow had already
  curved.
- **`shadow_color`** — the reference's `shadowColor`. Ours was a constant near-black, right
  on a light surface and too heavy on some dark ones.
- **`force_material_transparency`** — no background, no tint, no shadow. For a bar over an
  image or a video, where the chrome should be the controls and nothing else. It
  **overrides** the background and the elevation rather than arguing with them, because a
  caller asking for transparency has already decided and should not have that decision made
  conditional on whatever theme is in force.
- **`toolbar_opacity`** and **`bottom_opacity`** — the contents fade, the surface stays.
  Independent, as the reference's are: that is what a collapsing header does to its title
  while its background holds, and the `bottom` (a tab strip, usually) fades on its own
  schedule. Both are **group** opacities, so overlapping children do not darken where they
  overlap.
- **`actions_padding`** — the actions as a group. An icon button's own hit area already
  reaches the bar's edge, so a design that wants the *glyphs* inset had nowhere to say so.
  The reference added this in its 3.27 line for the same reason.

`shadow_color`, `surface_tint` and `shape` are on `AppBarTheme` as well, so an application
sets them once.

## Two things the tests found

**A shadow is as wide as the thing casting it.** The first version of the test helper looked
for the widest rectangle and found the shadow — it reported the bar's surface as a black at
22% alpha. Shadows are `blur > 0`; the helper says so now.

**A clipped subtree is drained into a composited `Layer`.** So a bar with a shape paints
nothing at the scene's top level, and a helper that only looks across the top concludes the
bar drew nothing at all. That is what the second failure was. Worth knowing generally: any
test that asserts on primitives has to walk into layers once clipping is involved.

## Still missing on the bar

Grouped by why, because the reasons are not the same:

**Needs scroll notifications reaching the bar** — `scrolledUnderElevation` and its
`notificationPredicate`. The bar has to be told that content has scrolled under it, and
nothing carries that signal yet.

**Needs a builder-based slot** — `primary`, for the reason milestone 394 records: the
reference splits the top padding between `Scaffold.primary` and `AppBar.primary`, and our
eagerly-built slots cannot.

**Straightforwardly next** — `flexibleSpace` (a widget behind the toolbar, which is what
makes a collapsing header possible at all), `iconTheme` / `actionsIconTheme`,
`toolbarTextStyle`, `excludeHeaderSemantics`, `automaticallyImplyLeading`,
`automaticallyImplyActions`, `clipBehavior`, `animateColor`.

**Platform** — `systemOverlayStyle`, which sets the status bar's icon brightness and is a
message to the system rather than a property of the widget.
