# Jalon 54 — Reachable animation layer + the demo's transitions on top of it

Milestone 53 laid down the `frus_core::animation` layer (physics, curves,
driver), but **an application could not reach it**: the demo depends on
`frus-widgets`/`frus-shell`, which did not re-export those types. So the driver
(`AnimationController`) stayed theoretical. This milestone makes it usable **and**
proves it end to end in the real app.

## A more ergonomic, reachable `AnimationController`

- **`AnimationController::spring_to(target, spring, velocity)`**: animates the
  current value towards `target` through a spring seeded with the current
  velocity — the path for **interruptible transitions started by a gesture** (a
  drag settling, a back navigation carried by the finger's momentum), without the
  caller having to build and box a `SpringSimulation` by hand.
- **`AnimationController` implements `Default`** (a `[0,1]` controller at rest),
  so it can live in a `#[derive(Default)]` app model.
- **Re-export** of the whole layer through `frus-widgets` (so apps can reach it
  without depending directly on `frus-core`). `frus_core::Status` (animation
  progress) is renamed **`AnimationStatus`** there so it does not shadow the
  interaction `Status` (paint state).

## The demo drives its transitions with `AnimationController`

A reminder of frus's Elm model: **the app owns its animation state and advances
it in `tick(dt)`** (the brief's option "B", adapted). The demo drops its ad-hoc
Euler integrator (`spring_step`) and its manual bookkeeping
(`nav_progress`/`nav_velocity`, `BackGesture.settling`) in favour of the shared
driver:

- **Screen transition**: an `AnimationController` pushed from `0 → 1` by
  `spring_to` (a near-critical spring, `k=220, c=30`). `tick` samples it, and the
  view reads `nav.value()` at paint time.
- **Back gesture**: tracked by the finger during the drag, then a **spring settle
  seeded with the finger's momentum** (`spring_to(target, spring, velocity)`) —
  the closed form handles the overshoot; the controller stops dead at the edge
  (`[0,1]`), which **commits** (`1`) or **cancels** (`0`) the pop.

The shared spring is now expressed once (`nav_spring()` → `SpringDescription`),
instead of being scattered as constants passed to a stepper.

## Validation

- `frus-core`: **37 tests** (+`spring_to`).
- `frus-widgets`: **122 tests**; `frus-demo`: **15 tests**, including
  `back_gesture_flick_commits_pop` — a quick flick **still pops** the screen after
  the migration (the driver reaches the target and commits).
- `cargo build --workspace` with no warnings; the demo ran without panicking (the
  transition and the gesture both work).

## Architectural note

frus does **not** build a shell-side controller registry driven by `Command` (the
brief's other variant): its "the app advances its animations in `tick`" model
reaches the same goal — animation state retained, kept out of the pure view, read
at paint time — with no new machinery in the shell. The framework's interaction
animations (hover/focus/opacity/value/scroll) stay handled by the `Runtime`; any
eventual migration of those to `Simulation` would change the feel and will happen
under goldens (see milestone 53).
