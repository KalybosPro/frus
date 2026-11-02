# Milestone 731 — BottomNavigationBar

## Objective

frus had `BottomBar`, the current design's navigation bar — a pill under the selected icon,
every label shown — and `BottomAppBar`, a bar of actions. It did not have the classic bottom
navigation bar that a great many applications are designed around and that people ask for by
name: items the same width with the selected one in colour, or a *shifting* bar where the
selected item widens, only its label shows, and its colour spreads across the bar.

## What was done

- **`BottomNavigationBar::new(current_index, on_tap)`** with `BottomNavigationBarItem`s (an icon,
  a label, an `active_icon`, a `background_color` for a shifting bar, a `tooltip`). The selection
  is the caller's: the bar shows what it is told and reports the item tapped; `on_tap` takes a
  closure returning `()` or a message.
- **Two kinds** (`BottomNavigationBarType`): `Fixed` and `Shifting`, fixed for three items or
  fewer and shifting for more unless told — the reference's rule.
- Every setting the reference has: `elevation` (8), `background_color`, `icon_size` (24),
  `selected_item_color` / `unselected_item_color`, `selected_font_size` (14) /
  `unselected_font_size` (12), `selected_label_style` / `unselected_label_style`,
  `show_selected_labels` / `show_unselected_labels` (unselected labels show on a fixed bar, not on
  a shifting one), `landscape_layout` (`Spread`, `Centered`, `Linear`) and
  `use_legacy_color_scheme`. The colours resolve as the reference's do: the caller's, then the
  theme's, then the kind's (`primary` in light and `secondary` in dark, 54 % black / 70 % white
  unselected for a fixed bar; `surface` for a shifting one).
- **The animation**: one controller per item, moved over 200 ms along a fast-out-slow-in curve
  when the selection changes. It drives the shifting tile's flex (1 → 1.5), the colours, the
  label's font size from the unselected size to the selected one (in a line as tall as the
  selected size needs, so the icons do not move), the label's fade, and the
  padding that keeps the icons level as labels come and go. A shifting bar also starts a splash
  of the new item's colour at the item's centre, reaching the bar's far corner; once spread, it
  becomes the bar's colour.
- **`BottomNavigationBarTheme`** under `theme.widgets.bottom_navigation_bar`.
- **`Scaffold::bottom_navigation_bar(widget)`**: the bottom slot, ahead of a bottom app bar, with
  the bar's height known to the shell (the floating button sits above it, an extended body runs
  under it). As with the other bottom bars since milestone 418, the scaffold *tells* the bar
  about the system's bars instead of padding it: the bar's colour runs behind the gesture bar and
  its items keep clear of it, and of the side intrusions in landscape.
- Each tile is an ink well with the tab role in the semantics tree, its label followed by where
  it is among the others ("Search, Tab 2 of 3"), and whether it is selected — announced whether
  or not the label is drawn. The semantics sit **on the ink well itself** (a crate-private
  `InkWell::semantics`): on a wrapper around it, a screen reader's activation found no message
  on the node it activated and did nothing.
- The widget gallery has a *BottomNavigationBar* demo (a fixed bar, a shifting one), and the
  guide's navigation page shows one in a scaffold.

## Built from widgets

The bar is a component made of rows, ink wells, opacity and scale, over a canvas that paints the
elevation, the colour and the splashes. Nothing in it needed a new primitive, and it is
transparent to the tree through `forward_transparent!` like `Scaffold` and `AppBar`.

## Found on the way

- **The test driver did not see text under an opacity or a transform.** `Driver::texts`, which
  `Tester::shows` and `Tester::tap` read, only looked at the scene's top level, and such text is
  drawn in a layer. A fixed bar's unselected labels, scaled down, were invisible to a test — and
  so was any application's text under an `Opacity`. It now goes into the layers, leaving out
  those drawn at no opacity, which show nothing.
- **A stage never moved the animations components drive.** `Stage::advance` stepped every
  family of retained motion the shell does except `tick_animations`, so a picture of a component
  animating was a picture of where it started. It now ticks them, as the shell's loop does.
- **Scaled-down text loses glyphs.** The first version scaled the label with a `Transform`, as
  the reference does; rendered, the unselected labels came out with letters missing ("Home"
  drawn as "Hme"). The bar now animates the font size instead — frus lays text out at any size,
  so it is as smooth and stays sharp. The glyphs lost under a scale are a renderer issue of their
  own, **not fixed here**.

Checked by eye: the fixed and shifting bars rendered in the light and dark themes, and a shifting
bar photographed 50 and 100 ms after its selection changed — the splash spreading from the new
item, its label fading in and the old one out.

## Tests

- In `bottomnavbar.rs`: the shifting flex, where a splash starts, the padding that keeps the
  icons level, a change of selection moving one item out and the other in with a splash that
  becomes the bar's colour, and a fixed bar not splashing.
- In `tester.rs`, through a whole application: tapping items changes the page, the bar is along
  the bottom edge, a shifting bar shows only the selected label (the others still reached by what a
  screen reader hears, "Library, Tab 3 of 4"), and widens the selected item to 1.5 shares.

## Divergences

- The reference scales the label with a transform; frus changes its font size (above). They look
  the same; the font size stays sharp.
- The reference keeps the selected icon a little larger than the others when it is given
  `selectedIconTheme` sizes; frus has a single `icon_size` for now.
- The reference's `mouseCursor` and `enableFeedback` are not separate settings: the ink well's
  cursor and feedback are frus's defaults.
