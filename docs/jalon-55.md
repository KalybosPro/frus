# Jalon 55 — Relayout boundary cache (retained layout on top of taffy)

The first brick of the **retained layout** called for in `docs/prior-art.md` (§1) —
and a prerequisite of the two remaining engine items (per-phase "dirty" lists,
targeted invalidation).

## The problem

frus rebuilds the widget tree every frame (Elm). Until now, **every layout root**
(`build_ui`, each scrollable, each navigation screen, each overlay, each
virtualised list item, each stack layer) restarted taffy *from scratch*:
`Layout::new()` → `build_layout` (allocating the whole taffy tree) → `compute` →
`absolute_rects`. But the geometry depends **only** on the tree's *style* and
*structure* and on the parent's *constraints* — **not** on colours or text, which
only affect painting. A hover, a blinking caret, an animating colour or opacity →
identical layout, yet fully recomputed.

## The solution: a per-root cache, indexed by identity

A new module `frus-widgets/relayout.rs`: `LayoutCache` remembers, per root
(`WidgetId`), `(signature, constraints, rectangles)`. At each root:

1. **A layout signature** (`layout_signature`): a 64-bit hash of the subtree that
   follows `build_layout`'s branching **exactly** (scrollable/navigator/list/stack
   = a leaf; portal = the anchor only), mixing each node's `Style::layout_hash`
   (new — geometric fields hashed by bit pattern) with its child count. Colours,
   text and messages are **excluded**.
2. If both the signature **and** the constraints are unchanged → the **rectangles
   are reused** and taffy is not called. Otherwise, recompute and remember.

The result is **bit-for-bit identical** to the full computation: on a *hit* we
return the rectangles we would have produced. Only the performance changes. The
worst case of a hash collision (astronomically improbable at 64 bits) is one
frame of frozen layout — never a crash.

The **7 layout sites** in `ui.rs` go through the cache (main root, scrollable,
screen, overlay, list item, stack layer, `LayoutBuilder`), each under a distinct
identity. The cache lives in the `Runtime` behind a `RefCell` (interior
mutability: `build_ui` only holds a `&Runtime`). At the end of the frame,
`end_frame()` **evicts** the roots that were not touched (widgets that have gone)
and freezes `(hits, misses)` diagnostic counters.

## Why this is the right foundation

- **Non-regressive**: identical output, proven by the 122 existing tests,
  unchanged.
- **A real gain**: during any colour/opacity/hover animation (the most frequent
  case), the signature is stable → taffy is **skipped entirely** each frame. The
  same goes for scrolling (offset ≠ layout) and during a screen transition (the
  screens are static, only the paint offset moves).
- **A prerequisite of the phases**: "the signature changed" *is* a root's "layout
  dirty" bit — the basis of the future `build → layout → paint → composite`
  pipeline with separate "dirty" lists.

## Validation

- `frus-widgets`: **129 tests** (+7: 6 unit tests of the cache — stable/changing
  signature, hit/miss, eviction — and 1 end-to-end through `build_ui`: frame 2
  reuses the root, a resize recomputes it).
- The whole existing suite green (output unchanged): `frus-core` 37, `frus-demo`
  15, shell 7, layout 3, gpu 4, text 2.
- `cargo build --workspace` with no warnings; the demo ran for 8 s without
  panicking (scrolling, transitions, overlays and the stopwatch all continuous) —
  the cache active in the hot path, with no `RefCell` borrow conflict.

## Limits / what's next

- The cache retains the **result** (rectangles) per root; it does not yet retain
  the taffy tree itself (no per-node `mark_dirty` within a root). A tiny change in
  a large root recomputes the whole root — the next step, if needed, is a
  persistent taffy tree reconciled by identity.
- Next milestone (§1): **frame phases + separate "dirty" lists**
  (`build → layout → paint → composite`), with each `Msg`/`Command` setting the
  narrowest possible bit — the relayout cache being its "layout" half.
