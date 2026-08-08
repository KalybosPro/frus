# Milestone 183 — `Steps` indicator: clickable markers

## Analysis

`Steps` (milestone 182) showed a wizard's progress but stayed **passive**: you could not click a
marker to **go back** to an already-visited step (to review/correct). Material's stepper makes its
step headers tappable — that was the gap.

## Technical decisions

- **An overlay of click zones, the rendering untouched.** `Steps` paints itself (connectors,
  markers, labels) with no children — and I did not want to break that pixel rendering. `on_tap`
  adds **one** row of **transparent** hotspots (one per step, the size of a marker) laid over it:
  it draws nothing but catches clicks and keyboard focus. So the `form_wizard` golden stays
  **identical** (verified without regenerating).

- **Exact alignment through `SpaceBetween`.** The hotspots are a
  `Flex::row().justify(SpaceBetween)` of boxes of diameter `MARKER_D`. Across the full width,
  `SpaceBetween` puts box `i`'s centre at `R + i·(W − 2R)/(n − 1)` — **exactly** the painted
  markers' `center_x` formula. The click zones therefore coincide precisely with the discs, with
  no hardcoded coordinates and no second calculation to maintain.

- **`Steps` becomes generic.** Carrying an `on_tap(|usize| Msg)` forces a `Steps<Msg>` (instead of
  milestone 182's non-generic `impl<Msg> Widget for Steps`). Each hotspot is a private
  `Hotspot { label, message }` widget: `on_click` emits the index, `focusable`, `Role::Button`
  semantics (the step's label). Without `on_tap`, `children` is **empty** → no overhead,
  milestone 182's behaviour preserved.

## Implementation

- `steps.rs`: `Steps<Msg>` (+ a `children` field), the `on_tap` builder (which builds the hotspot
  row), `center_x` moved into an `impl<Msg>` with no `'static` bound (called from `paint`), the
  private `Hotspot` widget.
- The rendering (`paint`) and `center_x` unchanged: the geometry is shared between the painted
  markers and the hotspots.

## Verification

- **Unit**: `on_tap_overlays_clickable_hotspots` — with no `on_tap`, no children; with it, a row
  of three hotspots each emitting `Msg::Go(i)` and focusable. Milestone 182's tests
  (`current_is_clamped_to_last`, `markers_reflect_progress`) **green**.
- **Golden** `form_wizard` **unchanged** (the test rerun without `FRUS_UPDATE_GOLDENS`): the
  overlay does not alter the rendering.
- The `Steps` doctest (annotated `Steps<()>`) **green**.

## What's left

- **Locking future steps**: only allowing jumps to steps already reached (the application already
  filters by choosing which `Msg`s it emits, but a built-in mode would be handy).
- **Vertical orientation** (stacked steps, the content under the current one) — still an extension
  (see milestone 182).
