# Milestone 396 — What is behind the bar, and what colours what is on it

Two more of the reference's `AppBar` properties, and one of them needed something the
framework did not have.

## `flexibleSpace` — a layer, not a slot

```dart
/// This widget is stacked behind the toolbar and the tab bar.
```

An image, a gradient, a photograph the title sits on. It is what makes a collapsing header
possible at all, and it is stacked **behind** everything the bar draws, filling the bar's
box.

The word that matters is *filling*. It does not make the bar taller: the bar is exactly as
tall as it was, however tall the widget would rather be. The test gives it a 400 px child on
a 64 px bar and asserts the title has not moved by so much as half a pixel.

The bar's own surface is painted underneath, so a translucent flexible space still tints
what the surface put there; a caller who wants only the image says
[`force_material_transparency`](milestone-395.md).

## `iconTheme` — and why it had to be a theme

The reference's `IconTheme` is an **inherited widget**, and that is not an implementation
detail. An app bar is handed a leading widget that is already assembled — an `IconButton`
with an `Icon` inside it, say. It cannot reach in and recolour the glyph. Passing a colour
down as an argument would only reach the widget it was given, not the one three levels
inside it.

So a colour that is meant to reach *every glyph in the bar* has to arrive as a **theme for
the subtree**. We had the mechanism already — `Themed::tweak`, from milestone 309 — and
what was missing was somewhere for it to land.

### `IconTheme`, in the theme

`Icon` read `theme.on_surface` and its own constructor. So the only way to recolour icons
in a subtree was to change the foreground colour, which would have recoloured the words
beside them too. `WidgetThemes::icon` is new — `color` and `size` — and `Icon` resolves both
as everything else in this framework does: **caller ?? theme ?? the framework's own**.

The size goes through `style_themed`, not just `paint`, so an app bar that makes its glyphs
smaller has them **take less room**, rather than the same room with a smaller drawing in it.

`AppBar::icon_theme` then delivers it with `Themed::tweak` over the leading slot and the
actions, setting both `widgets.icon` and `widgets.icon_button` so a bare glyph and one
inside a button answer alike.

The test is the point in miniature: a leading slot holding a plain `Icon`, recoloured
without the bar ever touching the widget.

### `actionsIconTheme`

Unset, the actions follow the bar's. Set, they part company with the leading slot — a back
arrow in the foreground colour beside actions in a muted one, which is the case the
reference's exists for.

## What the tests needed

`Primitive::Path` carries `fill: Option<Color>`, not `color` — an icon is a filled path, and
a path may have a fill, a stroke, both or neither. Worth knowing for any test that asks what
colour a glyph came out.

## Still missing on the bar

`scrolledUnderElevation` and `notificationPredicate` (nothing carries a scroll signal to the
bar yet); `primary` (needs the builder-based slot of milestone 394); `toolbarTextStyle`,
`excludeHeaderSemantics`, `automaticallyImplyLeading`, `automaticallyImplyActions`,
`clipBehavior`, `animateColor`; and `systemOverlayStyle`, which is a message to the system
rather than a property of the widget.
