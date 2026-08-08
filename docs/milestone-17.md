# Milestone 17 — Overlay / portal (floating menus, tooltips, modals)

Adds an **overlay** layer: showing content **above** everything, outside the
layout flow and not clipped by the parents.

## Mechanism

A [`Portal`] has an **anchor** (in the flow) and an optional floating
**overlay**. Like `Scroll`, the overlay is laid out separately and then
**deferred**:

1. `Widget::overlay() -> Option<(&dyn Widget, Placement)>`;
2. during the walk, the anchor (child 0) is painted inline; the overlay (child 1)
   is **collected** along with the anchor's bounds;
3. **after** the whole tree, `build_ui` handles the overlays: sub-layout (natural
   size), positioning, rendering **on top** (clip = the window). Their clickable
   zones are added last → so they **win** the hit-test.

`Portal::children() = [anchor, overlay]`: `find_widget`, the keyboard and
dragging also reach the overlay's content. Nested overlays are handled (a
processing loop).

## Placements

| `Placement` | Position | Extra |
|---|---|---|
| `Below` | under the anchor | drop-down menu |
| `Center` | centred in the window | dark **scrim** behind (modal) |
| `Tooltip` | above the anchor | shown **only while the anchor is hovered** |

The tooltip activates when the anchor's id (child 0, clickable) is the hovered
widget (`Runtime.input.hovered`).

## API

```rust
Portal::new(anchor).overlay(content, Placement::Below)     // floating menu
Portal::new(button).overlay(tip, Placement::Tooltip)       // tooltip on hover
Portal::new(trigger).overlay(modal, Placement::Center)     // modal + scrim
```

`Dropdown` is **rewritten on top of this mechanism**: its options now float above
the content (no more inline expansion from Milestone 16).

## Demo

- The `Dropdown` floats above the rest.
- The "Remove" button carries a **tooltip** on hover.
- A "Modal" button opens a **centred modal** (card + scrim + Close button).

## Tests

- `Portal::overlay` returns the content when one is supplied.
- A `Center` overlay draws a **full-screen scrim** plus its content on top.

## Limits (v1)

- Basic positioning (Below/Center/Tooltip); no auto-flip when the overlay
  overflows the screen, and no fine anchoring (start/end/aligned).
- Clicking **outside** a modal does not close it (no clickable scrim) — you close
  it through the dedicated button.
