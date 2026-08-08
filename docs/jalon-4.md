# Jalon 4 — Interactivity: events + state

Makes the UI **live**: widgets react to clicks and reflect changing state,
through a **message model** (in the style of Elm/iced).

## What ships

- **Generic `Widget<Msg>`**: a widget can emit a message on click
  (`Container::on_click(msg)`).
- **`Ui<Msg>` + `build_ui`**: building produces both the [`Scene`] to draw
  **and** a hit-test map. `Ui::hit(point)` returns the message of the topmost
  clickable widget.
- **Interactive loop** (demo shell): `State`, `view(&State) -> Widget<Msg>`,
  `update(&mut State, Msg)`. The window tracks the cursor and routes clicks.

## Architecture

```
state ──view()──► Widgets<Msg> ──build_ui──► Ui { Scene, hits }
  ▲                                             │  scene ─► frus-gpu ─► screen
  │                                             │
  └── update(msg) ◄── ui.hit(cursor) ◄── mouse click (winit)
```

Hit-testing reuses the driver's widget ↔ absolute rectangle pairing: clickable
zones `(Rect, Msg)` are collected in prefix order, and a click takes the **last**
zone containing the point (children, painted afterwards, are on top).

## Decisions

- **A message model** rather than callbacks: idiomatic Rust, avoids
  `Rc<RefCell>` and crossed borrows, and is testable. Widgets are parameterised
  by `Msg: Clone`.
- **`frus-widgets` does not depend on winit**: hit-testing takes a `Point`; the
  mouse is translated on the `frus-shell` side.
- **Coordinates**: physical pixels (the winit cursor and the viewport share the
  same space).

## Demo

A green bar-button; each click adds a coloured square to a row. Click →
`Msg::AddSquare` → `state.squares += 1` → rebuild → one more square. Proves the
event → state → rebuild → render loop end to end.

## Tests

- `build_ui` paints the right rectangles **and** maps the right clickable zones.
- `Ui::hit` returns the right message, and the **topmost** widget when they
  overlap.

## Limits (next milestones)

- **Hover/pressed** visual states, **focus** and **keyboard** all need widget
  identity to survive between frames → they come with **reconciliation**.
- The whole tree is rebuilt on every interaction (no diffing yet).
- Still no **text**.
