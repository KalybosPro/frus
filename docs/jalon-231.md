# Jalon 231 — Bounded DatePicker (days disabled outside [min, max])

## Analysis

`DatePicker` (a controlled monthly calendar) made **every** real day clickable — there was no way to
forbid dates. Yet a "real" date picker almost always has bounds: no past dates, a booking window, a
deadline. This milestone opens the **advanced date picker** domain with the first missing brick:
**disabled days**.

## Technical decisions

- **An internal `enabled` predicate, non-breaking.** The (private) `assemble` gains an
  `enabled: Fn(u32) -> bool` parameter. The existing public constructors (`new`, `range`,
  `range_dual`) pass `|_| true` — an **identical** rendering, with no public signature changed.

- **A disabled day = a muted, non-clickable cell.** The `Day` cell gains a `disabled` field: painted
  in a heavily muted `muted`, **without** a background or band, and its `message` is `None` (so
  `on_click`/focus are inactive). The **controlled** model is preserved: the widget decides nothing,
  it reflects the bounds.

- **A `bounded(min, max)` constructor.** A superset of `new` with two **optional, inclusive** bounds
  (`(year, month, day)` dates); a day is enabled iff `min <= date <= max`. A single bound (`None` on
  the other side) bounds on one side only.

## Implementation

- `frus-widgets/src/datepicker.rs`: the `disabled` field on `Day` + the muted rendering; `assemble`
  gains `enabled`; `new`/`range` pass `|_| true`; the new `bounded` constructor.

## Verification

- **Widget** `bounded_disables_days_outside_the_range`: a `[10, 20]` window in July 2026 → the 9th
  and the 21st not clickable (`on_click() == None`), the 10th/15th/20th clickable; with no max bound,
  the 31st stays clickable.
- **Golden** `date_bounded`: days 10–20 active (the 15th selected as a pill), the rest muted.
- Widgets 364; goldens: `date_range`/`date_range_dual` unchanged (the "all enabled" default).

## What's left

- Wiring `bounded` into the demo (the date screen) with a real window.
- Bounds in **range mode** (a bounded `range`) and arbitrary **blackout** days (a
  predicate/set).
