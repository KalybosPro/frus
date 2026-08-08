# Jalon 193 — Snackbar: animated exit + queue wired in

## Analysis

The demo only showed **one** notification at a time (`toast: Option<String>`), with no **exit**
transition: it vanished at once after 2 s. `SnackbarQueue` (milestone 185) existed but was not
wired in, and `ToastHost` (188) only knew how to fade in. The **animated exit** and the **really
wired queue** (several notifications following one another) were missing.

## Technical decisions

- **A "leaving" phase in the queue (framework).** `SnackbarQueue` gains a per-entry flag:
  `start_leaving()` marks the head as leaving, `is_leaving()` exposes it — the notification
  **stays visible** while the host plays its fade, then `dismiss()` removes it.
  `tick`/`dismiss`/`push` keep their API (milestone 185 intact); only the internal tuple grows to
  three fields.

- **`ToastHost::fade_out` (framework), mirroring `fade_in`.** Animates the group opacity towards
  **0** (through `AnimatedOpacity`, the existing animation layer) — the toast fades before it is
  removed. Both go through a shared `wrap_opacity(target, duration)`.

- **The queue wired into the demo, driven by timed commands.** `app.toast: Option<String>` becomes
  `app.snackbars: SnackbarQueue<String>` (`#[derive(Default)]` covers initialisation). The cycle is
  driven by three messages: `show_toast` enqueues and, if the notification becomes the head,
  schedules `ToastExpire` (~2 s) → `start_leaving` + schedules `DismissToast` (~0.3 s, the fade's
  duration) → `dismiss` then, if any notifications remain, reschedules `ToastExpire`. The rendering
  picks `fade_out` when `is_leaving()`, `fade_in` otherwise. Several `Save`s / sign-ups **stack**
  and go through one by one.

## Implementation

- `toast.rs`: `SnackbarQueue` — `start_leaving` / `is_leaving`, the `(T, f32, bool)` tuple.
- `toasthost.rs`: `fade_out` + the shared `wrap_opacity`.
- `frus-demo/src/lib.rs`: the `snackbars` field, `Msg::ToastExpire`, the `show_toast` /
  `toast_expire_after` helpers, the `Save`/`ToastExpire`/`DismissToast`/`WizardSubmit` arms, the
  rendering through `current()`/`is_leaving()`.

## Verification

- **Unit (framework)**: `leaving_phase_precedes_dismissal` (marks as leaving without removing,
  then removes); `fade_out_wraps_children`. The `queue_shows_one_at_a_time_and_expires` test
  (milestone 185) stays **green**.
- **Integration (demo)**: `snackbar_queue_orders_and_exits` — two notifications stacked, the head
  goes to leaving then yields to the next, the queue emptied. The other 17 demo tests stay green
  (18 in total).
- `cargo test -p frus-demo -p frus-widgets` **green, zero warnings**.

## What's left

- **Auto-ticking** (a real-time subscription) instead of timed commands — smoother for variable
  durations, but the timed approach is enough here.
- **A slide-out exit** (translation + fade) — through the animation layer (an animated
  translation), on top of the opacity.
