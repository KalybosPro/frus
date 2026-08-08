# Jalon 174 — Focus trap for open menus

## Analysis

**Modal** overlays (scrimmed: a modal, a drawer) already **trapped** keyboard focus —
Tab/arrows loop through their focusables while they are open (a focus scope). But **anchored**
overlays (`Placement::Below`) did not trap: an open **menu** (the column menu from milestone
172, a floating menu) let Tab escape to the page behind. Yet the expected keyboard pattern for a
menu is: focus **inside** the items, Escape to leave.

Trapping **every** anchored overlay would be wrong: a **tooltip** does not take the focus, and
an **autocomplete**'s list keeps the focus on the field (the arrows navigate the suggestions from
there). So the trap had to be **opt-in**.

## Technical decisions

- **Opt-in through `Widget::overlay_traps_focus()`.** A new trait method (default `false`,
  forwarded by `Box`/`Keyed`/`Responsive`). An anchored overlay only traps the focus if it
  returns `true`. **Modal** overlays always trap (unchanged).

- **Only `Menu` opts in (for now).** `Menu::overlay_traps_focus` returns `self.open`: an
  **open** menu traps its items; closed, it does not. `Escape` / an outside click closes it
  through `on_dismiss` (already in place) — no dead end. `Dropdown`, `Autocomplete` and tooltips
  keep the `false` default (behaviour unchanged).

- **A flag carried by the deferred overlay.** The deferred overlay tuple gains a `traps`
  boolean, read from the carrying widget when it is pushed; when it is placed, the focus scope
  starts if the overlay is **modal OR trapping**.

## Implementation

- `widget.rs`: `overlay_traps_focus()` (default `false`) + forwarders (`Box`, `keyed.rs`,
  `responsive.rs`).
- `menu.rs`: `Menu::overlay_traps_focus` = `self.open`.
- `ui.rs`: the `traps` boolean in the deferred overlay tuple (the type + `push` + `pop`); the
  focus scope starts if `modal || traps`.

## Verification

- **Unit**: `open_menu_traps_focus_in_its_items` — an open `Menu` traps Tab within its items
  ("one" → "two" → loop), the background is out of scope (pointer); a **closed** menu does not
  trap (Tab starts at the background).
- No regression: `modal_traps_tab_arrows_and_pointer_focus` (modals) and the
  autocomplete/tooltip tests stay **green** (they do not trap). `cargo test --workspace`
  **green**.

## What's left

- **`Dropdown` as a column menu**: having it return `overlay_traps_focus` from its open state
  would trap it too — to be enabled if the UX calls for it (single selection differs from an
  action menu).
- **Returning focus to the anchor** when the menu closes: the shell could restore the focus to
  the trigger (the full "roving focus" pattern) — an extension.
