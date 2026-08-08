# Milestone 68 — `ColorScheme`: consolidated roles (single source of truth)

The big colour piece of §5. Since milestone 58, roles had been piling up **flat**
on `Theme`; this milestone groups them into a real **`ColorScheme`** (the
Material 3 shape) without breaking a single one of the ~130 existing accesses.

## The architecture: the scheme is the source, the flat fields are derived

- **`ColorScheme`**: ~23 roles written by hand for light and dark — the
  `primary`/`secondary` family (plus containers and `on_*`), the surfaces
  (`background`, `surface`, `surface_variant`, **`surface_container[_high]`** for
  elevation, **`inverse_surface`** for toasts), the outlines
  (`outline[_variant]`), `error`, and **`scrim`**/**`shadow`** (with the alpha
  applied at the point of use). `lerp` role by role.
- **`Theme.scheme`** becomes the **source of truth**; the historical flat fields
  (`background`, `surface`, `primary`, `muted = on_surface_variant`,
  `border = outline`, …) are **derived views** through `Theme::from_scheme` —
  strictly identical values, so the widgets' API does not move. The theme's
  `lerp` interpolates the scheme and then re-derives the flat fields:
  consistency holds **even mid-fade** (pinned by `flat_fields_mirror_the_scheme`,
  also tested at `t = 0.37`).
- `focus`/`selection` remain frus's own interaction accents (outside the M3
  roles), passed to `from_scheme`.
- `Theme::from_scheme` is public: an app can supply its **own** complete scheme
  (the "everything must be customisable" rule); `from_seed` (HCT) will plug in
  here.

## Adoptions (the new roles are used immediately)

- **`scrim`**: the two hard-coded scrims (`rgba(0,0,0, 0.5·p)` for
  modals/drawers, `0.22·coverage` for the back screen during navigation) now go
  through `scheme.scrim.with_alpha(…)` — identical rendering (a black scrim), now
  themable.
- **`shadow`**: `Button`'s shadow (a hard-coded 35% black) goes through
  `scheme.shadow.with_alpha(0.35)` — likewise.
- **`surface_container_high`**: `Menu`'s rows (a **floating** panel) sit on the
  elevated surface instead of the base surface — Material's elevation tone,
  subtle.
- `secondary*` / `inverse_surface`: present in the scheme (palette completeness),
  with adoption suggested (chips → `secondary_container`, toasts →
  `inverse_surface`) as we go.

## Validation

- **241 tests**, all green — the existing theme tests pass (flat values preserved
  identically), plus the flat↔scheme invariant (3 themes, one of them a partial
  fade). A warning-free build; the demo did not panic.

## What's left (remaining §5)

`from_seed` (HCT) plugged into `from_scheme`; text decorations,
`letter_spacing`/`line_height`; `Alignment`; RTL (§14); progressive adoption of
`secondary*`/`inverse_surface`.
