# Jalon 123 — Extended showcase: Clip + InteractiveViewer

## Analysis

Three rendering milestones (J120 pixel tests, J121 shape clipping, J122
`InteractiveViewer`) had stacked up **without ever being shown**. This milestone
makes them *tangible*: the `frus-transforms` showcase gains a **shape clipping**
gallery and an **interactive viewport** — enough to *see* (and manipulate) the
clipping and the pan/zoom, beyond the headless tests.

## Technical decisions

- **Clipping made visible by contrast.** A **gradient square with crisp corners**
  is clipped by `ClipRRect(24)` and then by `ClipOval`: the difference from the
  original corners makes the clipping obvious at a glance.

- **A detailed interactive viewport.** A grid of dots on a gradient background
  fills a `260×180` viewport clamped to `0.5×`–`4×`. Dragging pans it, the wheel
  zooms (anchored at the cursor); at high zoom the content overflows and is
  **clipped to the frame**. Framed by a rounded `Container` so the frame reads.

- **The `view` stays pure.** The additions do not touch the Elm model: the same
  deterministic `update`, a subscription paused when stopped, and a `view` that is
  a pure function of the state. The viewport's transformation lives in the
  `Runtime` (retained state), not in the app.

- **Conventions.** Struct constructors (`ClipRRect::new`, `ClipOval::new`,
  `InteractiveViewer::new`); interface text in **English**.

## Implementation

- `crates/frus-transforms/src/lib.rs`: importing `ClipRRect` / `ClipOval` /
  `InteractiveViewer`; `gallery3` (two clipping tiles); `viewer` (the interactive
  viewport + its grid content); wiring into the scrolling column with headings;
  the title becomes "Transform · Clip · InteractiveViewer · AspectRatio".

## A fix discovered while rendering

Rendering the showcase offscreen revealed a **layout bug** in `InteractiveViewer`
(J122): any **sibling placed after** the viewport was overlapping it. The cause:
the viewport was not declared a **layout leaf** in `build_layout` (unlike
`Scroll`), so its subtree stayed inside the column's rectangles — and since the
walk places that subtree **separately** (a separate index), the main index went
out of sync for every following sibling. Fixed by adding `interactive()` to the
list of leaves; the regression is locked down by
`sibling_after_viewer_keeps_its_layout_position` (the sibling correctly follows
the 150 px viewport, with no overlap).

## Tests

- `renders_clip_shapes`: the `view` does emit a `ClipShape::RRect(24)` **and** a
  `ClipShape::Oval` (collected recursively from the layers).
- `sibling_after_viewer_keeps_its_layout_position` (frus-widgets): locks down the
  layout fix above.
- The existing guards hold: a transformed layer is emitted, and the content is
  **placed inside the viewport** (the blank-page guard). Suites green:
  `frus-transforms` 7, `frus-widgets` 222.

## Seeing it / running it

- Desktop: `cargo run -p frus-transforms` — then **drag** inside the interactive
  viewport and use the **wheel** to zoom; look at the clipping tiles.
- Android: an APK through `cargo-apk` (the same metadata as `frus-hello`).

## What's left

- Verification **on a real device** (desktop + Android) — the goal being to *see*:
  crisp clipping, smooth pan/zoom, hit-testing that follows.
- Two-finger pinch (touch) once multi-touch is in place.
