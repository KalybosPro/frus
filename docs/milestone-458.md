# Milestone 458 — A list tile's theme, and a sheet that takes a shape

The two items milestone 457 recorded, closed together.

## Every property a tile had was the caller's alone

`ListTileThemeData` is one of the larger tables in the reference, and there was no
`ListTileTheme` here **at all**. A list tile is the most repeated widget in most
applications — a settings screen is thirty of them — so *say it once* is not a convenience
there, it is the difference between a design decision and thirty copies of one.

`ListTileTheme` carries thirteen entries, all on the usual rungs: the tile's own word, then
the theme's, then the framework's. `tile_color`, `selected_tile_color`, `selected_color`,
`icon_color`, `text_color`, `shape`, `content_padding`, `title_style`, `subtitle_style`,
`title_gap`, `min_leading_width`, `min_height`, `dense`.

### One shape worth copying

`height()` had no theme in hand and is public. It now delegates to a `height_for(theme)`,
and `height()` is `height_for(&Theme::default())` — which is exactly what `style()` already
does with `style_themed`, one screen up in the same file. Following the file's own idiom
beat inventing a second one.

`text_color` sits **below** the selection in the chain and above the roles: a selected tile
takes `selected_color`, an unselected one takes `text_color` if anything named one, and
`on_surface`/`on_surface_variant` otherwise. That is the reference's order, and the useful
one — a theme that recolours its text has not thereby said what a selected row looks like.

## A sheet's corner was an expression nobody chose

```rust
let radius = theme.radius + 6.0;
```

Sixteen pixels, arrived at by adding six to something else. No caller could change it and
no theme could either, where the reference has `BottomSheetThemeData.shape`.

`BottomSheet::shape()` and `BottomSheetTheme::shape` / `radius` now resolve on the four
rungs, and the framework's own default is still `theme.radius + 6.0` rounding the **top**
pair only. That default is deliberately unchanged: Material 3 puts a bottom sheet's top
corners at 28, and moving it moves every picture that has a sheet in it — a change worth
making with the goldens read, not in passing. The number is now *reachable*, which is what
this milestone is for.

The bottom edge is flush against the window, so the framework rounds two corners and not
four. A caller naming a shape has taken that decision on: `ShapeBorder::rounded(28.0)`
rounds all four and two of them are off-screen. The builder says so.

## A sharp edge found by the test

`BottomSheet::body` is what wraps the panel, so anything the panel reads has to be said
**before** it. `open().shape(...)` compiles, runs, and does nothing — which is how the
first version of the test failed. `background` has always had the same trap and nothing
said so.

It is not fixed here — fixing it means the panel reading its style at build time rather
than at wrap time — but it is now written down in the test, where the next person to hit it
will be.

## The tests

- `a_theme_answers_for_every_tile` — the surface, the selected surface, the shape, the
  height and the padding, with the tile still outranking all of them.
- `a_sheet_takes_a_shape` — the framework's default unchanged, then the caller's, then a
  theme's radius, then a theme's shape over its radius.

**The goldens did not move.**
