# Jalon 172 — Table: column menu from the keyboard

## Analysis

After the header action widget (milestone 170), the natural next step was the **column menu**:
a header button opening a menu of actions (sort ↑/↓, hide…), reachable with **Tab** and driven
by arrows/Enter. The question: did the table need a new mechanism?

## Technical decisions

- **Nothing to build: composition is enough.** A [`Menu`](crate::Menu) (or a
  [`Dropdown`](crate::Dropdown)) dropped into `header_action` **is** a column menu. Verified end
  to end:
  - **Nested overlay rendered.** The layout walk detects `overlay()` on **any** visited node (a
    method forwarded by `Box<dyn Widget>`); a `Menu`'s floating menu, **child of a header cell**,
    is therefore collected and rendered over the grid, exactly as at root level.
  - **Keyboard.** The action button is already in the focus order (milestone 170); the `Menu`'s
    items are `focusable` → navigable; Enter/Space activates them (milestone 167's shell path).
  - **Dismissal.** `overlay_dismiss` bubbles up to `Ui::top_dismiss` (Escape / outside click).

- **No fake milestone.** Rather than a redundant mechanism, this milestone **locks the
  composition down** with a regression test and **documents** it as a recipe on
  `header_action`. The application drives the menu's open/closed state (consistent with the
  architecture: the table holds no transient state).

## Implementation

- `table.rs`: a "Column menu" doc note on `header_action` (the recipe + the guarantees).
- `goldens.rs`: `table_column_menu` (a "…" header button opening a floating menu).

## Verification

- **Unit**: `header_action_menu_opens_as_column_menu` — a `Menu` opened as a header action has
  its overlay **collected even when nested** (`top_dismiss` = the dismissal message) and its
  items **painted** above the grid.
- **Golden** `table_column_menu` **inspected**: a "…" button in the header, a floating "Sort
  ascending / descending / Hide column" menu over the data.
- `cargo test --workspace` **green**.

## What's left

- A **focus trap (modal)** for the open menu: the modal focus scope is handled at overlay level;
  to be confirmed for a column menu if the app wants to trap Tab inside the menu.
- The menu's **default item / ARIA role**: carried by `Menu`/`Dropdown`, not by the table.
