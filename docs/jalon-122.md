# Jalon 122 — `InteractiveViewer`: pan + zoom (pinch/wheel)

## Analysis

The natural outlet of the transformation stack (J112–J117) plus clipping (J121):
an **interactive viewport** where the user **pans** and **zooms** its child — the
brick behind a map, a detailed image, a floor plan, a diagram. Everything was
already there (a transformed and clipped layer, hit-testing by `M⁻¹`); what
remained was the transformation's **retained state** and the **gesture routing**.

## Technical decisions

- **A single layer does everything.** The child fills the viewport at scale 1, and
  is then wrapped in **one** `Primitive::Layer` carrying both the matrix `M`
  (scale + translation) **and** the clip to the viewport — compositing already
  applies the `M⁻¹` sampling and the clip test within the same instance.
  Hit-testing puts the point through `M⁻¹` (as `Transform` does).

- **The transformation is retained state, not app state.** `InteractiveView
  { scale, tx, ty }` lives in the `Runtime` (like the scroll offsets), indexed by
  viewport; absent = the identity. The `view` stays a pure function of the app
  state.

- **Pure, tested gesture maths.** `InteractiveView::pan` (the cursor pushes the
  content) and `zoom_at` (**zoom anchored at the cursor**:
  `t' = cursor·(1−f) + f·t`, with the scale clamped to `[min, max]`) are pure
  functions — the shell only calls them and reads back `matrix()`. The point of
  content under the cursor stays fixed while zooming.

- **Shell gestures, with tap/pan disambiguation.** A drag (mouse **or** finger) →
  `Drag::Pan`, engaged only beyond `TOUCH_SLOP`: so a plain tap passes through to
  the child (a button inside the viewport stays clickable). The wheel → a **zoom**
  anchored at the cursor (~1.1×/notch), with the bounds read from the widget.
  `interactive_at(point)` locates the topmost viewport.

- **A bounded size is required.** Like `Scroll`, the viewport needs a size
  (`width`/`height` or `flex`) or it collapses — see the "Scroll sizing" gotcha in
  [CONTRIBUTING.md](../CONTRIBUTING.md).

## Implementation

- `frus-widgets`: the `interactive` module — `InteractiveView` (state + maths) and
  `InteractiveViewer<Msg>` (`min_scale`/`max_scale`, `width`/`height`/`flex`); the
  `Widget::interactive()` method, forwarded (`Box<dyn>`, `Keyed`, `Responsive`,
  animated); a `Runtime::interactive` field; a walk branch (`ui.rs`) emitting the
  transformed and clipped layer and putting `M⁻¹` on the hits; the `interactives`
  collection + `Ui::interactive_at`.
- `frus-shell`: a `Drag::Pan` variant (tap/pan by threshold); `pointer_down`
  starts the pan; `handle_drag` applies it; `MouseWheel` zooms on an interactive
  viewport.

## Tests

- `interactive` (unit, pure): the identity is neutral; `pan` offsets by the exact
  delta; `zoom_at` **keeps the point under the cursor fixed**; zoom **clamped** at
  `max`.
- `interactive` (the walk): the emitted layer **carries the matrix and the clip**
  to the viewport; after a pan, hit-testing **follows** (the old position misses,
  the new one hits) — proof that `M⁻¹` crosses the transformation.
- The whole workspace green: frus-widgets 221 (+6), frus-gpu 16, frus-core 90.

## What's left

- **Multi-touch pinch** (two fingers): the input model is single-cursor; the wheel
  covers desktop zoom, and touch pinch will come with 2-finger tracking.
- **Inertia** (a fling on the pan) and **bounding** the pan (a boundary margin).
- A showcase: an `InteractiveViewer` row in `frus-transforms`.
