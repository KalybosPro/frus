# Milestone 185 — Snackbar: action + queue

## Analysis

`Toast` (a transient notification) was only a **static card**: no **action** (the Material
snackbar's "UNDO", which lets you reverse the operation), and the application had to handle
**stacking** and **auto-dismissal** by itself. Two gaps for a real notification system: an optional
action, and a "one at a time" queue that expires on its own.

## Technical decisions

- **A generic `Toast<Msg>` + an action.** Carrying an action message forces `Toast` to become
  generic (previously a non-generic `impl<Msg> Widget for Toast`). `action(label, msg)` adds an
  **uppercased text button** (the private `ActionButton` widget) on the right, emitting `msg` on
  click (focusable, `Role::Button` semantics). With no action, `children` is empty and the
  rendering stays **identical** (the demo, which infers `Msg`, compiles unchanged). The card then
  lays out as a row (`justify: End`, `align: Center`) to place the action on the right; the text
  is still painted by `Toast` on the left.

- **A pure `SnackbarQueue<T>`.** In the spirit of
  [`Form`](../crates/frus-widgets/src/form.rs): no painting, just state. A
  `VecDeque<(T, seconds)>` whose **front** is the visible notification. `push(item, seconds)`
  enqueues; `tick(dt)` counts **the head** down and removes it on expiry (returning `true` if the
  visible notification changed) — Material's auto-dismissal **with no timer widget-side**, driven
  by the application's loop; `dismiss()` closes the current one (a click on the action) and yields
  its payload; `current()` gives the displayed one. `T` is the application's payload (text, kind,
  action message).

- **A clean separation.** The widget draws, the queue schedules. The application links the two:
  `queue.current()` → a `Toast`, `tick(dt)` each frame, `dismiss()` on the action.

## Implementation

- `toast.rs`: `Toast<Msg>` (+ `action`, `action_w`, `children`); the private `ActionButton`
  widget; `accent` moved into an `impl<Msg>` with no `'static` bound (called from `paint`);
  `SnackbarQueue<T>` (`new`/`push`/`current`/`tick`/`dismiss`/`is_empty`/`len`).
- `lib.rs`: `pub use toast::SnackbarQueue`.
- `goldens.rs`: `snackbar_action`.

## Verification

- **Unit**: `action_is_clickable_and_uppercased` (no action → no children; with one → a clickable,
  focusable "UNDO" button); `queue_shows_one_at_a_time_and_expires` (only one visible, the head
  counted down, handover on expiry, manual dismissal, an empty queue inert). The existing painting
  test stays **green**.
- **Golden** `snackbar_action` **inspected**: an accent-barred card, the text, the "UNDO" action on
  the right in the accent colour.
- `cargo test -p frus-widgets toast::` **green**.

## What's left

- A **close button (cross)** built into the widget, on top of the action — a Material variant.
- **Enter/exit transitions** (slide/fade) driven by the existing animation layer.
- **Positioning/stacking** (top, bottom, corners): already in the application's hands through an
  overlay.
