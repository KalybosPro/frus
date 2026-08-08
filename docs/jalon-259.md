# Jalon 259 — Application lifecycle contract

## Analysis

The framework handled the **surface**'s lifecycle (winit's `resumed`/`suspended`: (re)creating and
destroying the renderer/window — essential on Android where the GPU surface is invalid in the
background), but **exposed nothing to the application**. Unlike the established lifecycle callback, the
`Application` could not react to going foreground ↔ background (suspending a timer/sensor, persisting
before closing).

## Technical decisions

- **A `Lifecycle` enum**: `Resumed` (foreground, interactive), `Inactive` (visible but unfocused),
  `Paused` (background, the surface lost), `Detached` (closing imminent).
- **An `Application::on_lifecycle(state)` hook** (default: nothing).
- **Notified on transitions only.** The shell remembers the current state (`lifecycle`) and only calls
  `on_lifecycle` on a **change** (`set_lifecycle`).
- **Wiring**: `resumed` → `Resumed`; `suspended` → `Paused`; the new `exiting` → `Detached`;
  `WindowEvent::Focused(true/false)` → `Resumed`/`Inactive` **without** overwriting `Paused`/`Detached`
  (the foreground decides focus, the background/closing decides the rest).

## Implementation

- `frus-shell/src/application.rs`: the `Lifecycle` enum + `fn on_lifecycle`; the export in `lib.rs`.
- `frus-shell/src/app.rs`: the `lifecycle` field (initialised to `Detached`), `set_lifecycle`
  (change-tracked), calls in `resumed`/`suspended`/`exiting` and the `WindowEvent::Focused` arm.
- `frus-demo/src/lib.rs`: `on_lifecycle` logs the state and sets `background = Paused|Detached`; the
  stopwatch's **subscription** is guarded by `!background` → the timer **suspends** in the background and
  **resumes** on return (the framework stops/restarts the subscription by diffing).

## Verification

- **Desktop**: compiles; shell 27; demo (lib) 36.
- **On device** (Huawei STK-L21): the sequence **confirmed** in logcat going to the background then
  returning: `Resumed → Inactive → Paused` (pressing HOME) then `Resumed` (on return). The stopwatch no
  longer runs while `Paused`.

## Notes

- `Inactive` rests on `WindowEvent::Focused` (reliable on desktop; on Android we do observe `Inactive`
  right before `Paused` when going to the background).
- Not there yet: a `Hidden` intermediate state, nor state restoration **after the process dies**
  (distinct from the live-reload `save_state`/`restore_state`, which is dev-only).

## What's left

- Reworking the **Kanban scrolling**: a **horizontal** board scroll + a **per-column vertical** scroll
  (instead of milestone 258's 2D `Axis::Both` pan).
- An overflow sweep of the other screens; DnD polish (same-column reflow, vertical inertia, the
  `Card`/`Toast` shadow).
