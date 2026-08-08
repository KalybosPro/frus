# Jalon 262 — Overflow sweep across the screens (scrollable tables + wrapped text + vertical bodies)

## Analysis

After the Kanban board (milestones 258/260), a cross-cutting audit of the demo's screens found the
**same class of overflow** elsewhere (the target: ~393 logical px wide). Fixed here on the same
pattern: wide content → a dedicated **scroller**; long text → **wrapped**; a tall body → a **vertical
scroll** (like `settings_screen`).

## Fixes per screen

- **Data table** (`data_screen`) — *severe*: a table of fixed columns (~610 px) → a **bounded scrollable
  region** `Scroll { axis: Both, flex: 1 }` (columns in X, rows in Y — a scrollable table, not a page
  pan). The hint and the focused detail moved to `.wrap()`. The **body** is `flex(1)` so the table region
  fills the height (otherwise it falls back to its base size, leaving a large gap below).
- **Editable grid** (`grid_screen`) — *severe*: an editable table (~644 px) → the same bounded scrollable
  region + a `flex(1)` body; the hint `.wrap()`.
- **Charts** (`charts_screen`) — *high*: the hint `.wrap()`; the body (the charts + the companion ≈
  550-650 px) wrapped in a **vertical scroll** (`Scroll::new().width(width).flex(1.0)`).
- **Wizard** (`wizard_screen`) — *medium*: a **responsive** field width
  (`(width - 48).clamp(240, 360)`, capped) instead of a fixed 360 px; the summary text `.wrap()`; the
  body in a **vertical scroll** (useful with the keyboard open).
- **Home** (`todo_screen`) — *low*: the icon showcase (~360 px) exceeded the card (~305 px) → a
  **horizontal scroll** of fixed height (52 px, the row's).

## The decision: a bounded scrollable region vs a page pan

A **table** (Data/Grid) is a 2D grid: a region scrollable in X **and** Y, **confined** to the table (the
rest of the screen fixed), is idiomatic. That is distinct from the 2D **page pan** rejected for the
Kanban board (milestone 260), where the whole page slid on the diagonal.

## Implementation

- `frus-demo/src/lib.rs`: `data_screen`, `grid_screen`, `charts_screen`, `wizard_screen` (+
  `wizard_input` takes a `field_width`), `todo_screen`.

## Verification

- **Desktop**: compiles; demo (lib) 36.
- **On device** (Huawei STK-L21), confirmed by screenshot:
  - **Data table**: the table fills the height (5 rows + the "1–5 of 12" pagination), **scrolls** (a
    horizontal bar → Score/Level reachable), and the hint shows on **2 lines**; the detail and the
    summary fixed below. The hardest case (pagination) — validated.
  - **Home**: the icon showcase scrolls **horizontally** (a bar under the row), no more card overflow.
  - **Editable grid** borrows the **same** pattern as Data table (a `Scroll{Both, flex}` region + a
    `flex(1)` body + a `.wrap()` hint) → identical behaviour. `charts`/`wizard` use the standard vertical
    pattern (`settings_screen`).
- Screens already safe (audited) left intact: `settings_screen`, `journal_screen`, `about_section`,
  `drawer_menu`, and `todo_screen`'s body (already a scrolling `Scaffold`).

## What's left

- **Per-column vertical** Kanban scrolling (the full pattern).
- Vertical drag inertia; a "scrollable screen" helper if the pattern recurs again.
