# Milestone 260 — Kanban scrolling: a deliberate horizontal axis (end of the 2D pan)

## Analysis

Milestone 258 had fixed the board's overflow by making it scrollable in **2D** (`Axis::Both`). A user
remark, and a fair one: a page does not scroll freely on the diagonal — scrolling is **deliberate, per
axis**, through a scroller configured for that axis. The established pattern for a board (Trello style):
a **horizontal scroller** for the **row of columns**, and a **per-column vertical scroller** for its
cards — two distinct scrollers, not a 2D pan.

## Technical decisions

- **The board = a horizontal scroller.** `Scroll { axis: Horizontal, width: viewport, flex: 1 }`: the
  row of columns scrolls **left/right** only; vertically, the content is **bounded to the viewport** (the
  columns align at the top). No vertical component to the gesture → no more diagonal panning.
- **Per-column vertical scrolling: deferred.** The full pattern (each column = an independent vertical
  list) requires giving the columns a **defined height** (otherwise the inner `Scroll` collapses — see
  the `Scroll` sizing gotcha). The demo's columns **fit** vertically today; so we keep the horizontal
  scroller alone and note per-column vertical scrolling as a follow-up (a layout change to the `Kanban`
  widget + regenerating the goldens).

## Implementation

- `frus-demo/src/lib.rs`: `board_screen` — `Axis::Both` → `Axis::Horizontal`.

## Verification

- **Desktop**: compiles; demo (lib) 36.
- **On device** (Huawei STK-L21): **confirmed by screenshot** (on a clean relaunch) — a **horizontal**
  scrollbar, the columns **aligned at the top** (no vertical panning), scrolling reveals "To do / Doing /
  Done", the hint on 2 lines. *(A first shot showed a remnant of the previous screen: a plain transition
  artefact from taps chained too quickly; it goes away on a clean relaunch — not a bug.)*
- The `kanban`/`kanban_rich` goldens render the **widget** directly (unchanged) → unaffected.

## Notes

- Drag/scroll coexistence unchanged: dragging a **card** reorders (the drag armed before the touch-scroll
  fallback, milestone 254); dragging an **empty area** scrolls horizontally.

## What's left

- **Per-column vertical scrolling** (the full pattern): a defined column height + an inner vertical
  `Scroll`, tested by filling a column with cards; regenerating the Kanban goldens.
- An overflow sweep of the other screens; DnD polish (same-column reflow, vertical inertia, the
  `Card`/`Toast` shadow).
