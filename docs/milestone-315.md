# Milestone 315 — The widget the last three milestones kept asking for

Milestone 313 found it and wrote it down: four of the framework's own buttons hold **one
glyph**, and a `Button` is sized for a word — 64 px wide before its label is measured, 24 px
of room either side. It gave them `min_width(40).padding(8)` and moved on. Milestone 314
found three more (the pagination strip, a table's column menu, a footer's picker) and did the
same thing again.

Seven call sites patched the same way is not seven call sites with a quirk. It is a missing
widget, and the reference has had it all along.

`IconButton` is 40 × 40, circular, no fill, the glyph in `on_surface_variant` — and, since
the bundled icon set has thirteen shapes and an application draws more than thirteen things,
it takes either:

```rust
IconButton::new(IconName::Close).label("Delete task").on_press(Msg::Delete)
IconButton::glyph("←").label("Back").on_press(Msg::Pop)
```

A button that only accepted what happens to be bundled would send exactly these call sites
back to `Button`, which is the problem this widget exists to end.

## The label is the point

An icon says nothing to a screen reader. `Button` gets its accessible name for free — the
label is the button — and an icon button has no text at all, so `label` is what makes it
announceable. That is what the reference's `tooltip` does, and it is why the parameter is
called out in the module's first paragraph rather than left as one builder among thirteen.

Where nothing is given and the content is a *glyph*, the glyph itself is announced. That is
worse than a name and much better than silence.

## What changed on the outside

The framework's own one-glyph buttons are icon buttons now: the date picker's month arrows
(which gained real chevron paths instead of the `‹ ›` characters), the navigation bar's back
arrow, and the stepper's plus and minus. In the demo, the task rows' delete cross and the app
bar's menu button.

Four variants, following the reference: standard (nothing but the glyph), filled, tonal,
outlined. Selected takes the accent, disabled greys out and goes inert, and seven settings
are the caller's or the theme's through `IconButtonTheme`.

## Verification

1054 tests (8 new), clippy silent, rustdoc clean, seven date-picker goldens re-blessed and
read — the arrows are bare chevrons now, which is what the reference's date picker header
has.

## Left

- **No toggle pair.** The reference's `isSelected` can swap the icon itself
  (`selectedIcon`), which is how a play/pause button is built. Here `selected` changes the
  colour and the glyph stays.
- **The icon set is thirteen shapes.** Every glyph call site above is a character standing in
  for a vector — a back arrow, a minus, an ellipsis — and characters depend on the bundled
  font covering them.
- **No tooltip.** `label` is announced but never shown; the reference's `tooltip` is both.
