# Milestone 43 — Adaptive layout (navigation & master-detail)

The second storey of responsiveness: beyond the primitives (milestone 42),
**screen structures that change shape** according to the [`SizeClass`].

## Batch A — `NavRail` + `BottomBar`

The two presentations of a single-selection main navigation, sharing the API
`new(selected, on_select).item(icon, label)` (the "icon" is a text glyph):

- `BottomBar` — a horizontal bar at the bottom (phone), items sharing the width.
- `NavRail` — a vertical rail on the left (tablet/desktop), fixed-width items.

An internal `NavItem` leaf paints the selection pill (`primary` background), the
glyph and the label, centred; hover and selection themed at paint time.

## Batch B — `NavScaffold` (the adaptive skeleton)

```rust
NavScaffold::new(size_class, selected, on_select)
    .destination(icon, label)…
    .body(content)
```

**Automatically** picks the presentation according to the class: **BottomBar** in
Compact (body on top, bar at the bottom — a column), **NavRail** in
Medium/Expanded (rail on the left, body on the right — a row). The `NavScaffold`
**is** itself the flex container; the body is wrapped in a `flex(1)` panel that
fills the rest. `body()` finalises (only one arm builds the navigation, so
`on_select` is moved only once).

## Batch C — `TwoPane` (master-detail)

```rust
TwoPane::new(size_class).ratio(0.36).show_detail(flag).list(a).detail(b)
```

**Side by side** in Expanded (proportional widths through `flex_grow` = `ratio` /
`1 - ratio`), a **single pane** otherwise (the list, or the detail when
`show_detail` — the app sets it to `true` when "navigating"). `detail()`
finalises.

## Infrastructure

A new `Widget for Box<dyn Widget<Msg>>` impl (delegating everything): it allows
an **already boxed** widget to be composed where an `impl Widget` is expected
(e.g. `Flex::child`) — indispensable for wrapping the `TwoPane`'s panes.

## Demo

Home moves under a `NavScaffold` (destinations **Tasks / Stats / About**): a rail
on the left when large, a bar at the bottom when narrow. The **Stats** section is
a master-detail `TwoPane` (metrics list | detail pane), side by side when large,
a single pane with a back action when narrow.

## Tests

`NavRail`/`BottomBar` (index emission, selection, item flexibility),
`NavScaffold` (column+bar in Compact, row+rail in Expanded), `TwoPane` (two
proportional panes in Expanded, a single one otherwise).

## Limits (v1)

- No **drawer** (Material's 3rd tier): bar ↔ rail only.
- `TwoPane` only switches to side-by-side in Expanded (the threshold is not
  configurable).
- Destination-based navigation is independent of the existing route stack (back
  gesture / push-pop) — the two coexist in the demo.
