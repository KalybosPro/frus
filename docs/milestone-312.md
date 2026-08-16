# Milestone 312 — The chip was a pill with nothing to say

`Chip` had two builders, `new` and `on_remove`, and painted itself as a **stadium** filled
with `muted` at 20 % opacity, its label at a hardcoded 15 px. It could not be pressed
(`on_click` returned `None` unconditionally), could not be selected, carried no icon, and
nothing about it could be changed.

The reference's chip is a **32 px rounded rectangle with an 8 px radius**, a `label_large`
label in `on_surface_variant`, and a 1 px `outline_variant` outline over no fill at all.
Selected, it fills with `secondary_container`, drops the outline, takes
`on_secondary_container` for its label and shows a checkmark. A pill is a different
component; the shape was Material 2's, and everything around it in this framework is
Material 3.

## The four chips are one chip

The reference has four classes — assist, filter, input, suggestion — and reading their
defaults side by side, what separates them is not shape. All four are the same box with the
same metrics. What differs is **affordance**: whether it can be selected, whether it carries
a leading icon, whether it can be deleted.

So there is one `Chip` with those as builders, which is the same set:

```rust
Chip::new("Draft")                                              // assist
Chip::new("Unread").selected(on).on_press(Msg::Toggle)          // filter
Chip::new(name).leading(IconName::Star).on_remove(Msg::Drop)    // input
```

Four types whose only difference is which builders you are allowed to call would be four
ways to write the same thing and one more thing to choose between.

## What the numbers are

Read out of `chip.dart` and `filter_chip.dart` rather than remembered: `_kChipHeight` 32,
radius 8, elevation 0, padding 8 on every side, label padding 8 either side, icons 18,
`side` of `outline_variant` and `Colors.transparent` once selected, checkmark on by default,
leading icon in `primary` at rest and in the fill's own colour when selected.

Every one of them is settable per call and through `ChipTheme` — thirteen fields, the
pattern from milestone 309: one `Option` per builder.

## Painted, not laid out

The label and the leading glyph are painted by the chip rather than being child widgets,
because their **colour follows the chip's state** and a child built before the theme is
known cannot be told what colour to be. That means the chip sizes itself: `style_themed`
measures the label and works out the width, and the padding is what places the one real
child — the delete cross — after it.

The cross is a child because it is a **separate hit target**. Pressing a chip and deleting
it are two gestures, and a positional click inside one target would make the boundary
between them invisible to everything except the code that computed it.

## Two things this turned up

**The cross was invisible.** Built with `Color::TRANSPARENT` and lerped toward `on_surface`
by the hover progress, it appeared *only* under the pointer — a delete affordance nobody can
find, and nobody can find it at all on a touch screen. It resolves its colour from the theme
now, at paint time, which is also what lets it follow the chip's state.

**The builder-order trap, again.** The cross carries a copy of the chip's style and state, so
`.on_remove(…).selected(true)` would have built the cross against the *unselected* colours.
Every builder rebuilds it. This is the second widget in two milestones with that shape —
`Tabs` had it for the same reason — and the rule is the same one: **a builder that assembles
a child must rebuild that child, or the order of the chain becomes part of the API.**

## Verification

1036 tests (9 new), clippy silent, rustdoc clean, and **four goldens re-blessed** — the
table screens, which put chips in cells. Each was read before it was accepted: grey pills
became outlined rectangles with the label in the muted role, and nothing else in the tables
moved.

While in `icons.rs`: four doc comments were still in French, from before the repository
settled on English. Translated.

## Left

- **No elevated chip.** The reference's variants can sit on `surface_container_low` with an
  elevation of 1; here a chip is always flat. `Card` has the shadow machinery already.
- **No avatar.** `leading` takes an icon from the bundled set, not an arbitrary widget, so
  an entry chip cannot show a photograph.
- **The checkmark does not animate in.** The reference grows it and slides the label aside
  over 150 ms; here the label is simply 18 px further along once selected.
- **Nothing is disabled.** A disabled chip in the reference is `on_surface` at 12 %, and
  this has no `enabled` at all — `Button` has one, so the two now disagree about whether
  that is a thing a widget has.
