# Jalon 245 — Demo: confirmation before a bulk delete

## Analysis

Bulk `Delete` (milestone 243) removes the checked rows **immediately** — irreversible and with no
safety net. The demo already has a **modal confirmation** pattern (clearing completed tasks: a `Portal`
+ a centred `Card`, dismissable by an outside click). This milestone applies the same pattern to the
data table's `Delete`, to close the domain off cleanly.

## Technical decisions

- **Reuses the existing pattern.**
  `Portal::new(screen).overlay(card, Placement::Center).dismiss(Msg::DataCancelDelete)` — identical to
  the clear confirmation, for a consistent UX.

- **The button no longer acts directly.** In the action bar, `Delete` now emits `Msg::DataAskDelete`
  (opening the modal) instead of `Msg::DataDeleteChecked`. The modal carries both outcomes: `Cancel`
  (`DataCancelDelete`) and `Delete` (`DataDeleteChecked`, the real deletion).

- **Navigation blocked while the modal is open.** `can_go_back` includes `!data_confirm_delete`, like
  the other modals — the back gesture/button does not navigate while the confirmation is open.

## Implementation

- `frus-demo/src/lib.rs`: the `data_confirm_delete` state; `Msg::{DataAskDelete, DataCancelDelete}`
  (+ `DataDeleteChecked` resets the flag); `data_confirm_content(count)` (a "Delete selected rows?" card
  + Cancel/Delete); `data_screen` makes the button emit `DataAskDelete` and wraps the screen in a
  `Portal` when the modal is open; `can_go_back` updated.

## Verification

- **Demo** `data_table_screen_…` extended: `DataAskDelete` opens the modal (nothing is deleted);
  `DataCancelDelete` closes it without deleting; `DataAskDelete` then `DataDeleteChecked` deletes the
  checked row and closes the modal.
- Demo 34; the shell compiles (the widgets/goldens unchanged).

## What's left

- A new widget domain: a `Tree` view (an expandable tree, selection) or a `Kanban` (columns + cards,
  drag and drop).
