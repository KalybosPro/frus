# Jalon 98 — `AnimatedContainer`: animated size (at layout)

## Analysis

J97 delivered animated colour (a **pictorial**, layout-free property). The other
half of `AnimatedContainer` is **geometric**: animating the **size**. That runs
deeper, because a **layout** property has to be known **at layout time** (taffy
reads `style()` *before* painting), not only at paint time.

## Technical decisions

- **Injection through a single `effective_style`.** The key: `build_layout` (which
  builds the taffy tree) **and** `hash_node` (the relayout cache's signature) both
  call `widget.style()`. Those calls are replaced by a shared
  [`effective_style(widget, id, runtime)`]: the widget's `style()`, with its
  **size replaced by the runtime's interpolated size** if the widget is animated.
  Since both paths share that source, they stay **automatically consistent** — and
  the signature **changes for as long as the size moves**, invalidating the cache
  frame after frame (relayout during the animation, then re-caching once it
  settles). No divergence is possible.

- **Runtime: a size timeline.** A [`SizeAnim`] `{ current, from, to, elapsed }`
  per node, tweened by `advance_sizes` on the **same model** as value and colour
  (rebasing on a change, snapping on mount, the widget's curve and duration) —
  linear interpolation per component (width/height).

- **Identities aligned.** `build_layout` and `hash_node` now propagate the `id`
  through `child_id`, **exactly** as the paint walk does — indispensable for a
  node's animated size to land on the right rectangle.

- **`Container` API**: `.animated_size(width, height, duration, curve)`; the trait
  method `Widget::anim_size() -> Option<Size>` (the target) + forwarders. A box's
  opacity, colour and size share one `(duration, curve)`.

## Scope & limits

- A widget laid out **separately** (scrollable/stack/navigator/list = a leaf in
  `build_layout`) does not animate its size through this path (an accepted limit);
  normal flow containers do.
- An animated size **defeats the relayout cache during the animation** (by
  construction: the geometry changes every frame) — as it does in any comparable
  engine. Once settled, the cache resumes.

## Implementation

- `frus-widgets`: `Runtime` (`SizeAnim`, `sizes`, `anim_size`, `advance_sizes`);
  the `anim_size()` trait method + forwarders; `ui::effective_style` +
  `build_layout` (id/runtime); `relayout`
  (`rects`/`compute_rects`/`layout_signature`/`hash_node` threaded);
  `Container.animated_size`.
- `frus-shell`: `advance_sizes` in the loop (before `build_ui`, so the size is
  ready **at layout time**).

## Tests

- `animated_size_tweens_between_frames` (runtime): snapping on mount (20×20), a
  linear tween → 40×40 (halfway ≈ 30×30), forgetting a widget that has gone.
- `animated_size_drives_the_layout` (end to end): halfway through, the **painted
  background rectangle** measures ~30×30 — proof that the interpolated size
  crosses `runtime → effective_style → taffy → rects → paint`.
- The relayout cache: signatures and hits unchanged with no animation
  (`effective_style` = `style()`), so goldens and the existing suites are
  **intact**.

## What's left

- Animated padding/radius/margin (the same layout-injection mechanics).
- Named `AnimatedContainer`/`Opacity`/`AnimatedOpacity` widgets (sugar over
  `Container`).
- Generic typed `Tween`s; explicitly driven animations (a controller).
