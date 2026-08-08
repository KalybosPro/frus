# Jalon 263 — Per-column vertical scrolling: layout blocker + reorderables-inside-Scroll guard

## The goal

To complete the Trello pattern: each **column** scrolls its cards **vertically**, independently (the
board's horizontal scroll in milestone 260 + per-column vertical scrolling here).

## What happened

The attempt: wrapping each column's card list in a `Scroll { axis: Vertical, flex: 1 }`, with the columns
stretched to the board's height. The result: the inner `Scroll` **collapses** — the cards disappear
(their visible rectangle is null, so they are neither painted nor **registered as reorderable**).

**The cause (a frus layout limit).** A `Scroll` at `flex(1)` only gets a usable height if the **ancestor
chain** supplies it with a **defined** height. But here: the board `Row` (an `Auto` height) → the column
(`Auto`/`Percent`) → the `Scroll` at `flex(1)`. `align: Stretch` and `Percent(1.0)` **are not enough**:
the stretched height is not treated as a defined basis for the inner flex (the scroll's viewport is
computed correctly — 196×248 in the registry — but its content is cut to zero). frus lacks a "**fill the
available height then scroll**" primitive that `Scroll`/`Flex` do not yet offer reliably. So the attempt
was **shelved**.

## An important discovery (and a guard)

While instrumenting, a finding: **reorderables placed inside an inner `Scroll` were not being
registered**. That raised a serious worry — milestones 258/260 **wrap the board (with its cards) in a
horizontal `Scroll`**, and I had only re-tested finger dragging **before** that wrapping. Checked: a
dedicated test shows that **a board inside a horizontal `Scroll` does register its cards as
reorderable** (`>= 2`). So **258/260 did not break dragging** — the collapse was **specific** to a
**vertical** `Scroll` at `flex(1)` with no defined ancestor height, not to being inside a `Scroll` at
all.

The `reorderables_inside_a_scroll_are_still_registered` test is **kept** as a guard: it protects the
board-in-a-scroll's dragging against a future regression.

## Implementation

- `frus-widgets/src/ui.rs`: the `reorderables_inside_a_scroll_are_still_registered` test (a board wrapped
  in a horizontal `Scroll` → the cards still reorderable). The attempt on `kanban.rs` was **entirely
  reverted** (the column structure unchanged).

## Verification

- **Widgets 394** (including the guard); kanban 7. *(The doctests were blocked at **runtime** by SAC —
  os error 4551, an environment issue, not a regression: they compile.)*

## What's left

- **Per-column vertical scrolling**: to be reopened once frus has a reliable "fill-then-scroll"
  primitive (or through an **explicit column height** passed by the app as a stopgap).
- Vertical drag inertia.
