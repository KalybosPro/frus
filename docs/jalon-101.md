# Jalon 101 — Explicit animations: `repeat` / `stop` / `reset`

## Analysis

Unlike **implicit** animations (J95→100: the framework interpolates towards a
declared target on its own), an **explicit** animation is **driven by the app**:
it decides when to start, reverse, repeat and stop.

The infrastructure was **already in place**:

- [`AnimationController`] (frus-core) — a value clamped to `[lower, upper]`,
  `value()`/`velocity()`/`status()`,
  `forward`/`reverse`/`animate_to`/`fling`/`spring_to`/`drive`, and
  `tick(dt) -> bool`. Publicly exported.
- The **frame hook**: `Application::tick(&mut self, dt) -> bool`, called by the
  shell **every frame**; for as long as it returns `true`, the shell requests
  another frame. It is the Elm/iced "ticker".
- The **demo already uses it** (navigation transition, gesture settle).

So the app-owned pattern is complete: the app holds a controller in its state,
advances it in `tick()`, and reads `value()` in `view()` to drive a widget.

**What was missing**: **repetition** (looping). The conventional API is a
`repeat(reverse:)` on the controller — ubiquitous (pulsing, halos, driven
indicators). frus's controller could only play one cycle and then rest.

## Technical decisions

- **`repeat(period, reverse, curve)`.** Each cycle lasts `period`, shaped by
  `curve`. `reverse` = a round trip (`0→1→0…`); otherwise a sawtooth (`0→1`, jump
  to 0, `0→1…`). Implemented **inside the `tick`**: at the end of a cycle, instead
  of resting, the controller **restarts** a cycle (from the opposite edge if
  `reverse`, otherwise from the bottom). `is_animating()` stays true — so `tick()`
  keeps returning `true` and the frames keep flowing.

- **Shared start-up.** `animate_to` was split: an internal
  `start_interpolation` (which does not touch the loop mode) serves **both**
  `animate_to` (one-shot, which cancels `repeat`) **and** the cycle restart. The
  one-shot methods (`forward`/`reverse`/`fling`/`spring_to`/`drive`/`set_value`)
  **cancel** a running loop — switching to a one-off animation naturally leaves
  the loop.

- **`stop()` / `reset()`.** `stop` freezes the value and ends the loop; `reset`
  returns to the lower bound (`set_value(lower)`). They complete the explicit
  control.

## Implementation

`frus-core/animation/controller.rs`: a `repeat: Option<Repeat>` field;
`repeat`/`stop`/`reset`; `animate_to` refactored around `start_interpolation`;
`tick` restarts a cycle if a loop is active; loop cancellation in the one-shot
starts.

## Tests

- `repeat_never_settles_and_restarts` (sawtooth): over ~16 cycles, it stays
  animating throughout, reaches the top, and **falls back** (the value drops → a
  new cycle).
- `repeat_reverse_ping_pongs`: the value **rises and then falls** (a round trip).
- `stop_and_reset_end_a_repeat`: `stop` ends the loop (nothing left to tick);
  `reset` stops it and returns to `0`.
- The whole suite green: the existing (one-shot) animations are unchanged.

## Usage pattern (a reminder)

```rust
struct App { pulse: AnimationController }           // state

fn init(&mut self) -> Command<_> {
    self.pulse.repeat(1.0, true, Curve::ease_in_out()); // a round-trip loop
    Command::none()
}
fn tick(&mut self, dt: f32) -> bool { self.pulse.tick(dt) } // advance, request a frame
fn view(&self, ..) -> Box<dyn Widget<_>> {
    Container::new().opacity(0.4 + 0.6 * self.pulse.value()) /* … */
}
```

## What's left

- A named widget/adapter for the common cases (e.g. binding a controller to a
  property) — the implicit path already covers most needs.
- A typed `Tween::animate(controller)` (frus-core has `Tween`) to map the value.
- A dedicated demo of a `repeat` loop.
