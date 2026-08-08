# Jalon 188 — ToastHost: positioning, stacking, transition

## Analysis

`Toast` (milestone 185) knows how to draw itself and carry an action; `SnackbarQueue` schedules.
What was left was **placement**: every screen redid "a column aligned in a corner with a margin"
by hand (the demo: `column![Toast].justify(End).align(Center).padding(20)`). And there was no
**appearance transition**. The layer that anchors, stacks and animates notifications was missing.

## Technical decisions

- **A full-screen layer anchored in a corner.** `ToastHost` fills the available surface
  (`width/height: Percent(1.0)`) and aligns its toasts through a column whose `justify` (top/bottom)
  and `align` (left/centre/right) follow from [`ToastPosition`] (six corners). You place it as the
  **last layer of a `Stack`** above the interface; it lets everything through and only places.

- **Native stacking.** Several `toast(...)` calls stack in a column (a fixed gap) in the corner —
  no more ad-hoc layout app-side.

- **An enter transition through the existing layer.** `fade_in(duration)` wraps **each** toast in
  an [`AnimatedOpacity`](../crates/frus-widgets/src/animated.rs) (an implicit animated opacity) —
  a fade-in with no new mechanism. Optional: the default rendering (and the golden) stays at full
  opacity, deterministic.

- **The content stays app-side.** `ToastHost` does **not** decide what to display: the application
  passes the current toast(s) (typically `SnackbarQueue::current`) and handles the queue /
  auto-dismissal. A clean split of placement / scheduling / drawing.

## Implementation

- `toasthost.rs`: `enum ToastPosition` (+ `justify`/`align`); `ToastHost<Msg>`
  (`new`/`padding`/`toast`/`fade_in`); `impl Widget` (a full-surface column, with no painting).
- `lib.rs`: `mod toasthost` + `pub use toasthost::{ToastHost, ToastPosition}`.
- `goldens.rs`: `toast_host` (two toasts stacked at the bottom right).

## Verification

- **Unit**: `empty_host_has_no_children`; `position_maps_to_justify_and_align` (`BottomEnd` →
  justify End/align End; `TopCenter` → Start/Center);
  `stacks_multiple_and_fade_in_preserves_count` (two toasts, `fade_in` preserves the count).
- **Golden** `toast_host` **inspected**: "File uploaded" (success) above "Message archived" +
  "UNDO", aligned at the bottom right, stacked.
- `cargo test -p frus-widgets toasthost::` **green**.

## What's left

- An **exit transition** (fade/slide before removal): requires keeping the toast one frame longer
  by leaning on the queue's state — an extension on the `SnackbarQueue` side.
- **Keyboard/inset offsetting** (lifting toasts above the mobile keyboard) — through the existing
  `WindowInsets`.
