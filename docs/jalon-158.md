# Jalon 158 — Reordering: faithful ghost (text included)

## Analysis

The reorder preview (milestone 155) lifted a **solid card** with no content: you saw a
rectangle move, not the column. Milestone 155 had left the "ghost including text" in What's
left, blocked by one obstacle: **replaying** the header's primitives translated runs into
their **clip** — each primitive carries the inherited clip rectangle, which would crop the
ghost back to the source column.

## Technical decisions

- **Un-clipping a primitive.** A new `Primitive::with_clip(clip)` (frus-core): copies a
  primitive **replacing** its clip. The shell captures the grabbed header's primitives
  (`owner == id`), **translates** them (`translated(dx, −2)`) then **un-clips** them
  (`with_clip(UNBOUNDED)`) — the ghost shows in full, wherever it goes.

- **A faithful face, a clean fallback.** `draw_reorder_overlay` now receives the ghost's
  primitives: a drop shadow, the **face = the header's primitives** (background + text + sort)
  replayed, a `primary` edge on top. If the capture is **empty** (a degenerate case), a solid
  face serves as the fallback — the function stays pure and testable (the test passes `&[]`).

- **`Primitive` re-exported** by frus-widgets so the shell (which does not depend on
  frus-core) can name the type in the signature.

## Implementation

- `scene.rs` (frus-core): `Primitive::with_clip(&self, clip) -> Primitive` (all the
  variants).
- `lib.rs` (frus-widgets): the `Primitive` re-export.
- `app.rs` (shell): `paint_reorder_preview` captures + translates + un-clips the header's
  primitives; `draw_reorder_overlay(…, ghost: &[Primitive])` replays the faithful face (a
  solid fallback when empty).
- `goldens.rs`: the `table_reorder_preview` golden, rebuilding the shell's overlay (the source
  dimmed, the indicator, a faithful "Role" card).

## Verification

- **Unit** (shape, no GPU): `draw_reorder_overlay` with an empty ghost → the solid fallback
  (4 primitives with a target, 3 without); `Primitive::with_clip` covered by the rendering.
- **Golden** `table_reorder_preview` **inspected**: the "Role" header **dimmed** in place, a
  **lifted card edged in `primary` carrying the text "Role"** offset to the right, and the
  **drop indicator** at the target column's edge. The ghost faithfully reproduces the header,
  text included.
- `cargo test --workspace` **green**, with no warning.

## What's left

- An **animated shift of the neighbouring columns** (they part to open the drop slot) — the
  last piece of the reorderable-list effect.
- **Ghost opacity** (< 1) for a more "lifted" look: requires wrapping the captured primitives
  in a `Primitive::Layer { opacity }`.
