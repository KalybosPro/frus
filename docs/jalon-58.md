# Jalon 58 — Theme: baked-in Material state layers + extended M3 roles

A continuation of the design system (§5). frus's theme was a **flat bag of
colours** and each widget reinvented its interaction states by hand — `Button`
lightened on hover, darkened on press, and hard-coded its `Danger` variant
(outside the theme). This milestone introduces Material's **baked-in state rule**
and starts **widening the roles**, in a strictly additive way (the ~130 existing
flat field accesses stay intact).

## A state layer baked into the theme

`Theme::state_layer(base, on, status)` overlays the content colour `on` on top of
`base` at a low opacity, according to the state: **hover 8%**, **focus 10%**,
**press 12%**, taking the animated progressions into account
(`hover_progress`/`focus_progress`). This is the brief's "role→colour according
to `Status`" rule, **baked into the theme**: the widget stays declarative (it
supplies its base colour and its content colour; the theme decides the overlay),
and every widget shares the same feel for states.

## Extended M3 roles (light/dark written by hand)

Five roles added to `Theme`, interpolated by the theme fade (`lerp`):
`primary_container` / `on_primary_container` (soft tonal surfaces), `error` /
`on_error` (a themed danger), `outline_variant` (a muted outline). The existing
flat fields (`background`, `surface`, `primary`, `on_surface`, `muted ≈
on_surface_variant`, `border ≈ outline`, …) keep **exactly** their values: zero
visual regression across the 60+ widgets that use them.

## Adoption: `Button`

`Button` drops its ad-hoc state logic in favour of `theme.state_layer(base, on,
&status)`, and its `Danger` variant now references the `error` / `on_error` roles
instead of a hard-coded colour. So the roles and the state layer are **genuinely
used**, not dead infrastructure. `Button`'s tests (which check the click message,
not the colours) pass unchanged.

## Validation

- `frus-widgets`: **130 tests** (+1: `state_layer` — neutral at rest, hover pulls
  8% towards the content colour, press stronger than hover).
- Everything else green: `frus-core` 46, `frus-demo` 15, shell 7, gpu 4
  (offscreen readback), layout 3, text 2. `cargo build --workspace` with no
  warnings.
- Rendering is not observable under WSLg-root; `Button`'s change of appearance
  (correct Material states) is intentional and matches the spec, and is not
  pinned by a test.

## What's next (§5, towards a complete `ColorScheme`)

- Generalise adopting `state_layer` to the other widgets with a themed hover
  (checkbox, switch, list rows, menu items…), as we go.
- Complete the roles (`secondary`/`tertiary`, `surface_container*`,
  `inverse_surface`, `scrim`) and then group them under a real `ColorScheme`; add
  `TextTheme` (15 slots) with `TextStyle`, and `from_seed` (HCT) later.
- Wire `BoxDecoration::content_padding` (milestone 57) into taffy.
