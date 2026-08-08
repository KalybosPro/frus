# Milestone 88 — Frame phases & repaint boundary cache

## Analysis

§1/§0 recommend a **phased** pipeline (`build → layout → paint → composite`)
where each `Msg`/`Command` dirties only the narrowest bit. frus already had two
pieces:

- **A conditional BUILD phase** (the driver): the `view` is only rebuilt if the
  state or the size change (`build_dirty`) or if the app is animating. A pure
  interaction frame (hover, focus, scrolling, caret) **reuses the retained
  tree**.
- **The relayout cache** (milestone 55): taffy is only called again if a root's
  *style or structure* changes — the "layout signature" **is** the "layout dirty"
  bit.

**Painting** was missing: even an interaction frame repainted the **whole** tree
(rebuilding the `Scene` completely, reshaping all the text). This milestone adds
the *paint* counterpart of the layout cache.

## What was done

### `RepaintBoundary` (opt-in)
`Container::repaint_boundary()` marks a container as a **repaint boundary** (a
new `Widget::repaint_boundary` method). Opt-in by choice: you only pay for the
cache where the content is **static** and the gain is real — a boundary placed on
the evidence of profiling.

### The paint cache (`paintcache.rs`)
It retains, per boundary and from one frame to the next, the subtree's **painted
output**: primitives **and** interaction maps (hits, focusables, semantics…). On
a *hit*, those already-formed primitives are **replayed** (with the clip and
owner *baked in*) without repainting — no text reshaping, no decoration
rebuilding.

Since the `Runtime` is generic-agnostic (it has no `Msg`), the data is stored
**erased** behind a `Box<dyn Any>` and downcast back to its concrete
`BoundaryData<Msg>` in the driver (one `Msg` instance per app → the `downcast`
always succeeds).

### Cache correctness (two locks)
1. **Generation**: any rebuild of the `view` (state, theme, size) **bumps a
   generation**; a stale entry is no longer a *hit*. Since the tree is the **same
   object** for as long as `build` does not run, an entry from the current
   generation ⇒ an identical configuration.
2. **Signature**: it covers the rest — the `Status` of **every** descendant
   (hover, focus, animated value/opacity, caret…) **and** the subtree's absolute
   rectangles. An unchanged signature + generation ⇒ the painting would be
   **bit-for-bit identical** → so the cache is replayed. Time is excluded (see
   below).

### A safe scope (a foundation)
A boundary is only cached if its subtree is **flat**: the boundary and **all** its
descendants take the default walk branch (children in prefix order) — no
scrollable, navigator, virtualised list, `layout_builder`, stack, overlay or
`continuous` animation. That case consumes the rectangles in the **exact** order
of the walk, which guarantees a correct bit-for-bit signature and replay. Any
descendant with dynamic layout ⇒ **not cacheable**, with a safe fallback: repaint
in full. (A subtree that pushes an overlay or touches the modal focus scope is
not memoised either.)

Excluding `continuous` is what justifies excluding **time** from the signature: a
cacheable subtree has no time-driven widget, so its rendering does not depend on
the clock.

### Driver & pipeline
The walk now goes through `walk` (boundary: hit → replay; miss → `walk_node` +
capture) on top of `walk_node` (the complete walk, unchanged) — so **nested**
boundaries are cached too. The driver **bumps the generation** right after a
`view` rebuild.

## Demo

The main card's **static** "Tip" banner is wrapped in a repaint boundary: it is
replayed from the cache on pure interaction frames instead of being repainted
every frame.

## Tests

- `repaint_boundary_reuses_a_static_subtree_bit_identical`: frame 1 = a miss
  (capture); frame 2 = a **hit**, and the replayed scene is **bit-for-bit
  identical** to a full repaint; the interaction maps (hits) are replayed too.
- `repaint_boundary_invalidated_by_generation_bump`: after a rebuild (generation
  bumped) → a full repaint.
- `repaint_boundary_invalidated_by_interaction_change`: an animated hover on a
  descendant changes the signature → a repaint; once settled → reuse.
- `paintcache`: a hit under equal generation/signature, invalidation by
  generation, eviction of boundaries that have gone at the end of the frame,
  freezing the diagnostic counters.

The existing widgets set no boundary (`repaint_boundary()` = `false`) → so `walk`
always delegates to `walk_node`: behaviour **unchanged**, no regression in the
existing suites.

## What's left

- **GPU compositing** (layers rendered into a texture, reused without
  re-uploading): deferred — hard to validate on WSL's software GPU. Here only the
  (CPU) paint walk is short-circuited; the scene is still re-uploaded.
- **Non-flat** boundaries (scrollables/navigator/stack): outside this
  foundation's scope.
