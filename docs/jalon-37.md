# Jalon 37 — New widgets: Breadcrumb, Pagination, Skeleton

Three widgets, each highlighting a different aspect of the framework.

## Widgets

- **`Breadcrumb::new(on_select).crumb("Home").crumb("Settings")`** — a
  breadcrumb trail: clickable segments separated by "›"; the **last** one is the
  current page (highlighted, not clickable). Clicking the i-th emits
  `on_select(i)`.
- **`Pagination::new(current, total, on_select)`** — a page selector (pages are
  **1-indexed**): ‹ prev · a **window** of pages around the current one · next ›.
  Prev/next are **disabled** at the bounds. Clicking a page emits
  `on_select(p)`.
- **`Skeleton::new().width(w).height(h)`** — a loading placeholder whose
  intensity **pulses over time** (shimmer). It reuses the continuous clock
  (`Status::time` + `continuous()`): the framework redraws on its own.

## Demo

- A **`Breadcrumb`** "Home › Settings" at the top of the Settings screen
  (clicking "Home" pops).
- A **`Pagination`** (controlled page) and two animated **`Skeleton`**s in the
  "About" tab.

## Tests

- `Breadcrumb`: `N` segments + `N−1` separators; links clickable, the current one
  not.
- `Pagination`: the correct window (current ±2); prev/next bounded (disabled on
  the first/last page).
- `Skeleton`: `continuous() == true`; the painted opacity **depends on time**.
- 77 frus-widgets tests; demo and stopwatch did not regress.

## Limits (v1)

- `Pagination`: a simple window (no "1 … 5 … 20" ellipses).
- `Skeleton`: an opacity pulse (no sliding gradient, which would need a dedicated
  shader).
- `Breadcrumb`: no truncation or "…" for very long paths.
