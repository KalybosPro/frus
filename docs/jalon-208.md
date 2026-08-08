# Jalon 208 — Sub-region highlight on hover

## Analysis

Milestone 205 changes the **cursor** (a hand) when hovering a clickable sub-region, but the
sub-region itself did not react. The expected visual feedback (a halo under the hovered icon)
requires the widget to know the pointer's **local** position at paint time — which `Status` did not
yet carry.

## Technical decisions

- **The position in `Status`, driven by the same signal as the cursor.** A new
  `Status::hover_cursor: Option<Point>` field (the pointer's **absolute** position), filled
  **only** for the hovered widget. Every `paint` already knows its `bounds`: it converts the
  position to local itself — no need to propagate the rectangle down to `full_status`.

- **Reuses `cursor_icon` (milestone 205), no new machinery.** The shell sets `hover_cursor` exactly
  when `cursor_icon` answers `Some` (the pointer is over an interactive sub-region), and resets it to
  `None` otherwise. As a result, the halo appears where and only where the hand appears, and the cost
  (a repaint on movement) is **limited** to those sub-regions.

- **The status hash includes `hover_cursor`.** Damage tracking (`hash_status`) therefore repaints
  when the position changes — the halo follows / withdraws. Outside an interactive sub-region,
  `hover_cursor` stays `None`: no extra repaint, the existing frugality preserved.

- **`InputState.hover_cursor`** relays the position from the shell; `status_for` restricts it to the
  hovered widget.

## Implementation

- `frus-widgets/src/interaction.rs`: the `hover_cursor` field on `Status` and `InputState`;
  `status_for` restricts it to the hovered one.
- `frus-widgets/src/ui.rs`: `hash_status` hashes `hover_cursor` (quantised).
- `frus-shell/src/app.rs`: `update_cursor_icon` sets `hover_cursor` from `cursor_icon` and repaints
  on change.
- `frus-widgets/src/textinput.rs`: `paint` draws a discreet rounded halo behind the **clickable**
  suffix when `hover_cursor` falls on it.

## Verification

- `hovering_active_suffix_paints_a_halo`: hovering the suffix paints a `~28x28` rectangle (the
  halo); in the body or with no hover, none. Purely visual — clicking and the cursor unchanged.

## What's left

- Generalising the halo to generic buttons/chips, and reusing `hover_cursor` for sub-region
  **tooltips** (the value of a chart bar / point under the pointer).
