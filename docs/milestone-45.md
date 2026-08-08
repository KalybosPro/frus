# Milestone 45 — Advanced widget responsiveness

Three complements to adaptive navigation: a **side drawer**, **notification
badges** on the destinations, and two **new axes** of responsiveness
(orientation and height).

## Batch A — `Drawer` (side drawer, Material's 3rd tier)

`Drawer` completes `NavRail` (rail) and `BottomBar` (bar): a full-height panel
that slides in from the left edge over the body, with a scrim that closes it on
an outside click.

```rust
Drawer::new(open)
    .on_dismiss(Msg::CloseMenu)
    .panel(nav_list)   // the drawer's content
    .body(main_screen) // the background, always visible
```

Implementation: a new **overlay placement**, `Placement::Left`. The panel
(`DrawerPanel`, internal) has a fixed width (`DRAWER_WIDTH = 280`) and a
`Percent(1.0)` height; the `Left` overlay is computed with its **height
constrained to the window** (free width), so that the panel unfolds over the full
height. The scrim and click-to-close reuse the `Center` modal mechanism. When
closed, the `Drawer` emits no overlay at all (only the body is rendered).

## Batch B — Badges / counters on destinations

A navigation destination can carry a **notification counter**: a red pill at the
top right of the glyph, capped at `99+`.

```rust
NavRail::new(sel, Msg::Go).item("✉", "Mail").badge(5)
BottomBar::new(sel, Msg::Go).item("✉", "Mail").badge(5)
NavScaffold::new(class, sel, Msg::Go).destination("✔", "Tasks").badge(active)
```

`.badge(count)` decorates the **last** destination added; `count == 0` paints
nothing (the badge is hidden). The red is constant (an alert reads as red
whatever the theme).

## Batch C — Extra axes: orientation & height

`frus-core` gains:

- `Orientation { Portrait, Landscape }` with `Orientation::from_size(w, h)` (the
  convention: square → portrait), `is_portrait()`, `is_landscape()`;
- `SizeClass::from_height(h)` — the same thresholds as for width, to drive the
  **vertical** axis (a short window → `Compact` in height).

Re-exported from `frus-widgets` and `frus-shell`. The app composes these
primitives however it likes (the shell already provides `on_resize(w, h)`).

## Demo

- **Drawer**: a "☰" button in the header opens a drawer listing the sections
  (Tasks / Stats / About) plus a way into the settings; choosing a section or
  navigating closes it. The back gesture is neutralised while it is open.
- **Badge**: the `NavScaffold`'s "Tasks" destination shows the number of active
  tasks.
- **Orientation / height**: `on_resize` logs the orientation; in a **short**
  window (`from_height == Compact`) the tip is hidden and the list shrinks
  (200 px instead of 320).

## Tests

- `frus-core`: `from_height`, `Orientation::from_size`
  (portrait/landscape/square).
- `frus-widgets`: `Drawer` — an overlay only when open, scrim + full-height
  panel, no dismiss hit when closed; badge — decorates the right item, `99+` cap.
- `frus-demo`: the drawer toggles and closes when a section is chosen or on
  navigation; `on_resize` tracks the orientation.

## Limits (v1)

- No slide animation for the drawer (opening and closing are instant).
- `Placement::Left` only (no right-hand drawer) — trivial to add if needed.
- No *permanent* drawer (always visible in Expanded): the rail plays that role;
  the drawer stays modal.
