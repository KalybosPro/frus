# Jalon 38 — New widgets: Tree, ColorPicker, Timeline

Three widgets filling structural gaps (hierarchy, colour choice, chronology).

## Widgets

- **`Tree::new(on_toggle).node(id, depth, "src", expandable, open)`** — a
  **controlled** hierarchical tree. The application owns the structure and the
  expansion state, and passes only the **visible rows**, flattened. Each row is
  indented by its depth, with a ▸/▾ chevron for nodes that have children;
  clicking an expandable node emits `on_toggle(id)`. A good split (the widget
  renders, the app owns the tree — like `Toast`).
- **`ColorPicker::new(selection, columns, on_pick).swatch(colour)`** — a palette
  of swatches built on `Grid`. The selected swatch carries a ring; clicking emits
  `on_pick(colour)`.
- **`Timeline::new().event("Title", "detail")`** — a vertical chronology: each
  event is a dot joined by a continuous line, with a title + detail.

## Demo (the "About" tab, "Advanced options" section)

- **`Tree`**: a collapsible file explorer (`State.expanded: HashSet<u64>`,
  `Msg::ToggleNode`). The app flattens the visible nodes according to the state.
- **`ColorPicker`**: a palette of 6 colours (`State.picked`, `Msg::PickColor`).
- **`Timeline`**: the recent milestones.

## Tests

- `Tree`: expandable nodes are clickable (`on_toggle(id)`), leaves are not.
- `ColorPicker`: swatches; the selection adds a ring (focus border).
- `Timeline`: events; dots + texts painted.
- 80 frus-widgets tests; demo and stopwatch did not regress.

## Limits (v1)

- `Tree`: no row selection and no guide lines; the app entirely handles the
  hierarchy and the flattening.
- `Timeline`: a simple continuous line (no branches or per-dot coloured
  statuses).
- `ColorPicker`: the palette is supplied by the caller (no continuous HSL
  picker).
