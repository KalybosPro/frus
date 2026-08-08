# Jalon 205 — System cursor per sub-region

## Analysis

Hovering a clickable icon (a field's eye/✕ suffix, milestones 198/202), the pointer stayed an
arrow: nothing signalled "this is clickable". Browsers and mature toolkits change the cursor to a
**hand** over clickable zones. So a widget had to be able to request a cursor shape for a
**sub-region**, and the shell to set it on the window.

## Technical decisions

- **A local opinion, like `positional_click`.** A new trait method:
  `Widget::cursor_icon(local_x, local_y, width, height) -> Option<Cursor>`. It receives the pointer's
  **local** position within the widget's box and returns the desired shape, or `None` (no opinion →
  the shell keeps the default). Purely visual: it does not affect clicking.

- **Widgets stay independent of the windowing layer.** A small `frus_widgets::Cursor` enum
  (`Default` / `Pointer` / `Text`) is the exchange unit; the shell translates it to
  `winit::window::CursorIcon`. No widget depends on winit.

- **Recomputed on every move.** The shell (`pointer_move`) asks the hovered widget through
  `widget_rect` + `find_widget` (the same path as the positional click) and sets the cursor on
  **every** move — the sub-region can change without the hovered widget changing (the eye vs the
  field's body).

- **`TextInput`** returns `Pointer` over its **active** suffix icon (`on_suffix` set) through the
  existing `suffix_hit`; elsewhere, `None`. A purely decorative suffix does not change the cursor.

## Implementation

- `frus-widgets/src/interaction.rs`: the `Cursor` enum (exported from `lib.rs`).
- `frus-widgets/src/widget.rs`: the `cursor_icon` method (default `None`) + the `Box` forwarder.
- `frus-widgets/src/{keyed,responsive}.rs`: forwarders.
- `frus-widgets/src/textinput.rs`: the `cursor_icon` override (a hand over the active suffix).
- `frus-shell/src/app.rs`: `update_cursor_icon`, called by `pointer_move`, translates to
  `CursorIcon` and calls `window.set_cursor`.

## Verification

- `cursor_icon_is_pointer_over_active_suffix` (widgets): `Pointer` over the suffix, `None` in the
  body, `None` if the suffix is decorative. The rest of the workspace compiles and passes
  (forwarders).

## What's left

- **Highlighting** the hovered sub-region (a halo on the icon): needs the local hover position in
  `Status` — not plumbed through yet. A `Text` cursor over field bodies, `Pointer` over generic
  buttons/links, and tooltips reusing this same sub-region mechanism (chart bars/points).
