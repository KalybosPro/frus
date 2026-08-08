# Milestone 53 — Unified physics (`trait Simulation`)

The starting point of the **engine foundations** proposed in `docs/prior-art.md`
(§4). Until now, animation was **fragmented**: a hand-written spring curve
(`spring_ease`), an ad-hoc Euler integrator (`spring_step`), a separate scrolling
physics (`scroll_axis`), and per-widget springs in `runtime.rs`. Every kind of
motion reinvented its own maths; nothing generalised.

This milestone introduces the brick the brief calls "the most portable idea": **a
simulation is a pure function `time → value`**. Fling, scroll momentum and
sliding sheets can now share a single path.

## The `frus_core::animation` layer (pure, zero-dependency)

Placed in `frus-core` (the common base, with no rendering and no platform), so it
is usable by the widgets **and** by the shell.

- **`trait Simulation { x(t), dx(t), is_done(t), tolerance() }`** — the shared
  contract, with no mutable state and no upward borrow.
- **`SpringSimulation`** — a damped spring in **closed form**, with the three
  regimes chosen by the discriminant `c² − 4mk`: critical, overdamped,
  underdamped. `SpringDescription` describes the spring by `(mass, stiffness,
  damping)` or by a **damping ratio** (`with_damping_ratio`, `1` = critical). The
  critical regime is detected with a relative tolerance so that `ratio = 1` does
  not oscillate because of floating-point rounding.
- **`FrictionSimulation`** — momentum deceleration in closed form, with
  `through()` (calibrating the `drag` to pass through two points) and
  `final_x()`.
- **`ClampedSimulation`** — pins the **position** within `[min, max]` while
  letting the **velocity** carry on reporting (for scrolling).
- **`Curve`** — `[0,1] → [0,1]` shaping: `Linear`, `Cubic` (Bézier by binary
  search, with the `ease/ease_in/ease_out/ease_in_out` presets), `Interval`
  (which unlocks **staggered** animations for free), `Flipped`, and
  `CriticalSpring` (the step response that replaces `spring_ease`).
- **`Tween<T>` + `trait Lerp`** — a `[0,1]` driver animates any typed value
  (`f32`, `Color`, `Point`, `Size`).
- **`AnimationController`** — the **driver**: a clamped value + a `Status`
  (`Dismissed/Forward/Reverse/Completed`) + a `Box<dyn Simulation>` + `tick(dt)`.
  Everything it does — `forward`/`reverse`/`animate_to` (interpolating a curve
  over a duration) or `fling` (a physical spring) — is expressed as a simulation.
  **A single tick loop for everything.** This is the object the shell will
  instantiate by identity (`child_id`) in the next milestone; the view will read
  `value()` at paint time.

## Integration into the live path (parity proven)

`runtime::spring_ease` now **delegates** to `Curve::critical_spring()` (the same
`omega = 8`, the same floating-point operations): the shared layer becomes the
single source of truth for that curve, **without changing a pixel**. The tests
that pin the feel (`bottomsheet`, `drawer`) pass unchanged — so numerical parity
is verified automatically.

## Validation

- `frus-core`: **36 tests** (23 of them new, for the animation layer) — the
  springs (3 regimes), the friction, the clamp, the curves, the tweens and the
  controller are all covered, and `dx`↔`x` consistency is checked by finite
  difference.
- `frus-widgets`: **122 tests** green (`spring_ease` parity).
- `cargo build --workspace` with no warnings; the demo ran for 8 s with no
  regression (stopwatch and rendering continuous).

## What's left (remaining engine foundations, §1)

- A **relayout boundary cache** on top of taffy `(constraints, size, dirty)`.
- **Frame phases + separate dirty lists** (`build → layout → paint →
  composite`).
- **Wiring `AnimationController` into the shell**: a registry keyed by
  `child_id`, driven by `Command` (`animate` / `fling`), values read at paint
  time — then migrating scrolling and navigation from the ad-hoc Euler
  integrators to `SimulationController` (this changes the feel → to be re-pinned
  by goldens).
