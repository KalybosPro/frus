# Milestone 78 — Runtime inspector (§13, stage 1)

## Analysis

A continuation of the DX work (§13): "expose the diagnostic dump (§2) as an
overlay (tree + rects + ids). A developer who *sees* why their identity breaks on
reorder will stay." This is the widget inspector idea, stage 1: seeing the boxes,
pointing at a widget, correlating with a text dump.

## Architecture

- **`Widget::debug_name()`** — the runtime type name: the concrete type's name
  without its path or generics (`Container<Msg>` → `Container`). A default trait
  method: `std::any::type_name::<Self>()` accepts `?Sized`, and each
  implementation receives its **monomorphised copy** of the default body — zero
  impls to write across the ~60 widgets. Transparent wrappers (`Box`, `Keyed`,
  `Responsive`) delegate to their content.
- **Collection during `build_ui`**: the `Builder` gains an optional sink
  (`inspector: Option<Vec<InspectorNode>>`) plus a depth counter around each
  `walk`/`render_item` — the `(id, painted rect, name, depth)` nodes come out in
  paint order, overlays included (re-rooted at depth 0). `build_ui_inspected`
  exposes the `(Ui, nodes)` pair; `build_ui` is unchanged (a `None` sink, zero
  cost).
- **`inspector.rs`**: `node_at` (the deepest under the point), `dump_tree`
  (indented text: `Name  x,y  w×h  #id`), `paint_overlay` (outlines tinted by
  depth; the pointed-at widget = a primary scrim + a name/size/position/id card
  near the widget, bounded to the window, on `inverse_surface`). Ids abbreviated
  to 32 bits (in the card **and** the dump, so they correlate).
- **Shell**: **F12** toggles it (debug builds only, `cfg!`), and the dump goes to
  stderr on activation; the paint phase calls `build_ui_inspected` and paints the
  layer onto a **copy** of the scene (the retained scene is not polluted); cursor
  movement forces a redraw while the inspector is active (so the highlight
  follows, even over inert widgets).

## Decisions

- No retained `nodes` field in the shell: `build_ui` already runs on every
  painted frame, so the collection lives and dies with the frame.
- The long id (64 bits) made the card overflow (wrapping at the window's width —
  caught by the **golden**): abbreviated to 8 hex digits.
- Desktop first (F12); the touch activation gesture will come with the Android
  work.

## Tests (269 → 275)

- `collects_names_rects_and_depths` (concrete names, Keyed transparent, depths,
  the normal path unchanged), `node_at_picks_the_deepest`,
  `dump_tree_indents_by_depth`, `overlay_paints_outlines_and_hover_card`,
  `debug_names_are_short_and_delegated` (frus-widgets).
- `inspector_overlay_matches_golden` (frus-test): the complete overlay rendered
  and pinned as a PNG.

## The rest of §13

State-preserving hot reload (`subsecond`), a `cargo new` template, and for the
inspector: click-to-freeze selection, display of the retained state
(hover/focus/scroll), and a touch activation gesture.
