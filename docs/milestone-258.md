# Milestone 258 — Respecting the viewport: scrollable Kanban board + wrapped text (end of overflow)

## Analysis

Observed **on device**: elements **run off the screen**. Two concrete cases on the Kanban screen (a
phone ~393 logical px wide):
1. The **board** is a row of **fixed-width** columns (`COL_W = 220` × 3 + gaps ≈ 452 px) **with no
   scrolling** → the last column (or columns) overflows to the right, off-screen.
2. The **hint** is a **single-line**, unwrapped text → it overflows to the right ("…drag a card to
   mo—" cut off).

The framework must **bound content to the viewport** and make it **scrollable** when it exceeds it, and
**wrap** text.

## Technical decisions

- **A 2D scrollable board.** The board is placed in a
  `Scroll { axis: Both, width: viewport, flex: 1 }` that **fills the space** below the bar (the same
  pattern as the Settings screen: `Scroll::new().width(width).flex(1.0)`). Content wider **or** taller
  than the viewport → it scrolls. The `padding` is **inside** the scrolled content (the visual margin
  preserved).
- **Drag/scroll coexistence.** Unchanged and correct: on pressing a **card**, the shell arms
  `Drag::Reorder` (milestone 250) **before** the touch-scroll fallback (guarded by `drag.is_none()`,
  milestone 254); on pressing an **empty area**, it is a scroll. So dragging a card reorders, dragging
  the emptiness scrolls — the expected behaviour.
- **Wrapped text.** The hint uses `Text::wrap()` (already offered by the widget: `measure_wrapped` at
  the proposed width); placed in a screen-width `Container` → it wraps onto 2 lines instead of
  overflowing.

## Implementation

- `frus-demo/src/lib.rs`: `board_screen` — the board in
  `Scroll::new().axis(Axis::Both).width(width).flex(1.0)`, the hint `.wrap()` in a full-width
  `Container`; the `Axis` import.

## Verification

- **Desktop**: compiles; demo (lib) 36 (the reduce logic unchanged).
- **On device** (Huawei STK-L21): **confirmed by screenshot** — a horizontal scrollbar present;
  scrolling reveals "To do", "Doing", "Done" in turn (nothing left off-screen); the hint shows on **2
  lines**, not cut off.

## Notes

- The general rule ("bound to the viewport + scroll") applies beyond this screen; this milestone handles
  the **reported** case (Kanban). Sweeping the other screens on request.
- The `Scroll{Both, width, flex(1)}` pattern is the multi-axis counterpart of the vertical pattern
  already in use (Settings) — a good candidate for a "scrollable screen" helper if the need recurs.

## What's left

- Sweeping the other screens for any residual overflow (table widths, long labels…).
- A **lifecycle** contract (a state enum + an `on_lifecycle` hook, wired onto `resumed`/`suspended` +
  Android's `onPause`/`onStop`).
- Same-column reflow coverage; vertical inertia; the `Card`/`Toast` shadow on the theme.
