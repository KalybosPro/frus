# Milestone 261 — DnD polish: themed `Card`/`Toast` shadows + same-column reorder test

## Analysis

Leftovers raised in milestones 255/256: (1) `Card` and `Toast` painted their shadow with a **hardcoded
black** (`Color::rgba(0,0,0,0.3)`), whereas `Button` and the drag ghost (milestone 255) take theirs from
`theme.scheme.shadow` — inconsistent with the customisability rule; (2) the vertical reflow
(`reflow_reorder_cards`) had **no** test for the **same column** case (source and target in the same x
band).

## Technical decisions

- **Themed shadows.** `Card` and `Toast` take `theme.scheme.shadow.with_alpha(0.30)` instead of a literal
  black. Since `scheme.shadow` **is** black in the themes we ship, the rendering is **identical** — a
  de-hardcoding, overridable through the theme (the widgets become themable).
- **A same-column test.** It documents the correct behaviour of an in-column reflow: with the lifted card
  at the top and the insertion after the 2nd card → the card **above the line moves up** one notch
  (closing the gap), the one **below the line stays** (a net `+notch`/`−notch` shift of **zero**), the
  drop slot opening just above it. The neighbouring column does not move.
- **Vertical inertia: not adopted.** The horizontal spring (`reorder_x`) smooths the neighbouring
  columns' slide; the vertical equivalent would be pure **runtime** polish (not inspectable on a GPU
  here) with marginal benefit — set aside (see What's left) rather than adding unverifiable code.

## Implementation

- `frus-widgets/src/card.rs`: a `theme.scheme.shadow` shadow (the `Color` import dropped, now unused).
- `frus-widgets/src/toast.rs`: a `theme.scheme.shadow` shadow.
- `frus-widgets/src/reorder.rs`: the `same_column_reflow_lifts_upper_cards_and_holds_the_rest` test.

## Verification

- **Widgets 393** (+1); **goldens 77 unchanged** (the `Card`/`Toast` shadows do appear in goldens — the
  de-hardcoding is pixel-identical, no regression); no warnings.

## What's left

- **Vertical** inertia/spring for the cards' slide (parity with the horizontal) — runtime polish.
- **Per-column vertical** Kanban scrolling (the full pattern).
- An overflow sweep of the other screens (Data table, Grid, Charts, Wizard — the audit is done).
