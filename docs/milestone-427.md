# Milestone 427 — Every panel onto its rung, and two that had none

Milestone 426 built the container ladder and put on it the six call sites that were already
asking for a container role. This one finishes the sweep: the panels that were filled from
the **flat `surface`** where the reference names a rung, and the two navigation widgets that
painted no surface at all.

## The five that were on the flat surface

| widget | the reference | what it took |
|---|---|---|
| `Drawer` (`navigation_drawer.dart:740`) | `surfaceContainerLow` | `surface` |
| `BottomSheet` (`bottom_sheet.dart:1496`) | `surfaceContainerLow` | `surface` |
| `BottomAppBar` (`bottom_app_bar.dart:327`) | `surfaceContainer` | `surface` |
| `Dropdown`'s panel (`menu_anchor.dart:4035`) | `surfaceContainer` | `surface` |
| `Autocomplete`'s panel (`menu_anchor.dart:4240`) | `surfaceContainer` | `surface` |

The pattern behind the roles is worth stating once, because it is what makes them
memorisable rather than arbitrary. **A thing that slides in off the page takes the low
rung** — a drawer, a sheet, a banner, a card off the page. **A thing that is a distinct
area within the page takes the middle** — a menu, a navigation bar, a bottom app bar. **A
thing that is filled on purpose takes the top** — a filled card, a filled field. And the
one thing that covers everything, a dialog, takes `high`.

## The two that painted nothing

`NavigationRail` and `BottomBar` drew a hairline and let whatever was behind them show
through. A bottom bar sitting on a page was therefore **the page, with a line above it** —
which reads as a rule across the content rather than as a bar with its own surface, and
which is exactly the thing a screenshot would not obviously show as wrong.

The reference gives them **different** rungs, and the difference says what each is:

- `NavigationRail` → `surface` (`navigation_rail.dart:1202`). A rail stands *beside* the
  page, so it is the same kind of surface, not a thing laid on one.
- `BottomBar` → `surfaceContainer` (`navigation_bar.dart:1440`). A bar stands *on* the
  page, so it takes a distinct area within it.

Both gained a `background(color)` builder, and `NavRailTheme` gained `background_color` and
`bar_background_color`. The reference keeps a theme object per navigation widget with a
different default in each; this crate keeps one for the two, so the two surfaces are two
fields rather than two structs — said in the field's own documentation rather than left for
someone to trip over.

`BottomSheet` gained a `background(color)` builder and a `BottomSheetTheme` because it had
neither: its surface was a hard-coded read of `theme.surface`, with no way for a caller or a
theme to change it. `BottomAppBar` already had `color(…)`; it gained the theme half.

## The tests

`a_bar_and_a_rail_each_paint_the_rung_they_stand_on` asserts each widget paints a rect
covering its whole box, in the rung the reference names — and then asserts that the two
rungs are **not the same colour**, so the two assertions above cannot both pass by accident
on a scheme where they happened to coincide.

Removing the two fills makes both new tests report `None` rather than a wrong colour, which
is the old behaviour exactly: not a bar in the wrong tone, a bar with no surface.

`the_caller_and_the_theme_outrank_the_rung` covers the other half — the caller's word, then
the theme's, then the rung — on both widgets at once.

## Checked and deliberately not changed

Three widgets look like they should move and should not.

- **`Slider`'s inactive track.** The reference's `surfaceContainerHighest` is in
  `_SliderDefaultsM3Year2023` (`slider.dart:2204`) — the *superseded* defaults. The current
  `_SliderDefaultsM3` says `secondaryContainer` (`:2294`), which is what this crate already
  paints, and what an earlier milestone's test defends against the disabled fill.
- **`Chip` at rest.** The `surfaceContainerLow` at `action_chip.dart:305` is guarded by
  `_chipVariant == _ChipVariant.flat ? null : …` — it is the **elevated** chip's fill. A
  flat chip gets `null`, which is the transparent rest this crate already has.
- **`Switch`'s off track.** The reference does fill it with `surfaceContainerHighest`
  (`switch.dart:2246`) where this crate uses `outline`, but that is not a rung swap: the
  reference pairs that fill with a 2px `trackOutlineColor` (`:2251`) and an `outline`
  thumb (`:2212`), and changing one of the three alone would give a filled track with no
  edge. Recorded as its own step.

`surface_variant` is worth a line too: the reference deprecated it in favour of
`surfaceContainerHighest` (`color_scheme.dart:1228`). Nothing in this crate paints with it,
so the deprecation costs nothing here — but the role is still in the scheme, and a widget
reaching for it later would be reaching for a dead one.

## Still open

A detail the goldens surfaced: **a `Dropdown`'s closed trigger is built from the same row
widget as its options**, so it takes the panel's tone rather than a field's. The reference's
is an `InputDecorator` — a text field, filled or outlined. The two moved together here and
still look of a piece, but they are not the same thing and should not share a fill.

The switch's off-state colour model, above. And the role families the scheme is still
without: `surfaceDim` / `surfaceBright`, the tertiary five, `errorContainer` /
`onErrorContainer`, `inversePrimary`, `surfaceTint`, the `*Fixed` set.
