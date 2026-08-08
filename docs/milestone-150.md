# Milestone 150 — `Dropdown` / `Autocomplete` audit: bringing them up to standard

## Analysis

Both widgets worked but fell short of what their established counterparts offer:

- **`Dropdown`**: a **hardcoded** width (240 px), the selected option **not indicated**
  (neither highlight nor tick), a header and options **not focusable** (no keyboard), and
  **no tests**.
- **`Autocomplete`**: a hardcoded width (260 px) — not "customisable".

## Technical decisions

- **`Dropdown` rebuilds its tree.** It now stores its state (label, width, selected index,
  openness, options) and regenerates the header + menu (`rebuild`), which opens up the
  `width(px)` and `selected(index)` settings without breaking the API.

- **The selected option, done properly.** In the menu, the option at index `selected` is
  **highlighted** (a `primary`-tinted background) and **ticked** (a `Check` icon on the
  right) — as a dropdown button should be. The header's chevron becomes a **vector
  triangle** (no more dependency on the "▾" character).

- **The keyboard for free.** The header and options return `focusable` when they carry a
  message: the shell reaches them on Tab, opens / chooses on Enter, and the arrows walk the
  stacked options (the existing geometric navigation). No new logic.

- **`Autocomplete`: an adjustable width.** Since the field is rebuilt at each setting, its
  `on_input` callback becomes **shared** (`Rc`) so the rebuilt `TextInput` can recapture it;
  `width(px)` applies to the field **and** the suggestions. The suggestions were already
  focusable (keyboard OK).

## Implementation

- `dropdown.rs`: `Row` gains `width`/`selected`/`focusable` (highlight + tick + vector
  chevron); `Dropdown` stores its state and `rebuild`s; `width`, `selected`; **3 tests**
  (overlay open/closed, focusability, the option highlighted+ticked) — the module had none.
- `autocomplete.rs`: callbacks in `Rc`, the field rebuilt; `width(px)`; `Suggestion` gains
  `width`.
- `goldens.rs`: the `dropdown_menu` golden (an open menu, "Medium" highlighted + ticked).

## Verification

- **Unit**: `Dropdown` closed → no overlay, open → a 2-option menu whose 2nd emits
  `Select(1)`; header + options focusable; the selected option highlighted + ticked (a path
  + a tinted rect). `Autocomplete`: the existing tests green with the adjustable width.
- **Golden** `dropdown_menu` rendered and **inspected** (the header + chevron, the floating
  menu, "Medium" ticked in green). `cargo test --workspace` green.

## What's left

- **`Autocomplete`**: an **active** suggestion highlighted + keyboard descent from the field
  (down arrow), Material style; **highlighting the matching text**.
- **`Dropdown`**: opening/closing from the keyboard is handled by the app (a toggle
  message) — an Escape shortcut to close would be a plus.
