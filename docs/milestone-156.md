# Milestone 156 — Range slider (two handles)

## Analysis

`Slider` (a single `0..=1` cursor) did not cover a common need: picking an **interval** (a
min/max price, a date range…). A **two-handle** slider was needed.

The technical crux: two handles on **one** widget. The existing drag mechanism registers
**one** draggable rectangle per widget and delivers `on_drag(fraction)` — the absolute
fraction across the slider's whole width. How do you know **which** handle is moving?

## Technical decisions

- **One widget, the nearest handle.** Rather than a composite with two draggable children
  (absolute positioning + deltas, heavy), `RangeSlider` stays a **leaf widget** reusing
  `on_drag(fraction)` as is: the fraction moves the nearest handle. A deterministic rule that
  rules out crossing:
  - `f ≤ low` → the **low** handle follows;
  - `f ≥ high` → the **high** handle follows;
  - in between → the nearer one moves.
  At the extremes, the gesture **hands over** to the other handle (you keep dragging, the edge
  of the range extends); the handles never cross.

- **Controlled.** `on_change(low, high)`: the application receives the new interval and passes
  it back. `new(low, high)` **clamps** to `0..=1` and **reorders** (`low ≤ high`).

- **Rendering reusing `Slider`.** The same constants (`H`, `TRACK_H`, `THUMB`): a track, an
  **active segment** in `primary` between the two handles, two circular handles. `Semantics`
  `Slider` with the value "low%–high%".

## Implementation

- `slider.rs`: `RangeSlider<Msg>` (`low`, `high`, `width`, `on_change`); `new` / `width` /
  `on_change`; `Widget` (track + segment + 2 handles; `draggable`; `on_drag` → the nearest
  handle → `on_change(low, high)`).
- `lib.rs`: `pub use slider::{RangeSlider, Slider}`.
- `goldens.rs`: the `range_slider` golden (the interval `0.3..0.7`).

## Verification

- **Unit**: a drag near the low / high end moves the right handle (`(0.25, 0.8)`,
  `(0.2, 0.75)`); past the bounds, that side's handle follows and is **clamped**
  (`(0.2, 1.0)`, `(0.0, 0.8)`); `new(0.9, 0.1)` **reorders** into `(0.1, 0.9)`.
- **Golden** `range_slider` **inspected**: a grey track, a **green segment** between two white
  handles ringed in `primary`.
- `cargo test --workspace` **green**.

## What's left

- A **sticky handle**: remember the handle grabbed on press (drag state) so it stays selected
  even after crossing, Material style — requires an `on_drag` aware of the original handle (or
  two separate draggable handles).
- **Ticks / a discrete step** (`divisions`) and value **tooltips** on hover.
