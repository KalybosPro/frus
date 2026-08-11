# Milestone 278 — How fast was the finger going?

## The goal

Milestone 277 gave scrolling the right physics. This one fixes its **input**.

Every ballistic motion in the framework starts from one number: the speed of the
pointer at the moment it let go. A fling's distance, whether a swipe dismisses a
route, how far a pan coasts — all of it is that number, run through a curve. Until
now it was computed like this, in three separate places:

```rust
velocity.0 = velocity.0 * 0.5 + (dx / dt) * 0.5;   // an exponential average
```

Cheap, and wrong in the way that matters.

A finger does not move at a constant speed. It accelerates, it wobbles, and — the
case that ruins the estimate — it usually **slows down just before lifting**, because
lifting is itself a movement. An exponential average puts half its weight on the very
last sample, so it reads that deceleration as the gesture's speed. The user's thumb
says *throw*; the framework hears *nudge*, and the content barely moves.

Measured on the gesture the tests now encode — seven samples at 1000 px/s, then two at
125 px/s as the finger lifts — the old estimate read **~190 px/s**. The new one reads
**over 400**, and the platform-average strategy reads **956**.

## The shape

```
frus-core   VelocityTracker, VelocityEstimate, Velocity, PolynomialFit   the estimate
frus-shell  one tracker per gesture, and the gate that decides           the use
```

### Fitting a curve instead of smoothing deltas

`VelocityTracker` keeps the last 20 positions and, on release, **fits a quadratic**
through the ones that belong to the current motion, taking the fit's first-order
coefficient as the velocity. A quadratic sees acceleration, so a last-instant slowdown
moves the estimate a little instead of dominating it.

"Belong to the current motion" is three rules, and each earns its place:

- **a 100 ms horizon** — anything older is a different part of the gesture;
- **a 40 ms gap** ends the history — a pause means the finger stopped, and what came
  before it is a different gesture entirely;
- **40 ms since the last sample** means the finger is *resting*, and the release is not
  a fling at all, whatever it was doing a moment ago.

The fit itself is a least-squares solve: a QR decomposition by Gram–Schmidt, then back
substitution. It runs in `f64` even though frus is `f32` throughout — orthogonalising
powers of the abscissae loses precision quickly, and this is the number every fling
depends on. `PolynomialFit` also reports an **r-squared**, so a caller can tell a clean
gesture from a noisy one; nothing uses it yet, but the estimate carries it.

### Two strategies, because platforms disagree

Some platforms do not regress at all for scroll flings. They take a **weighted average
of the last three sample-to-sample velocities**, deliberately leaning on the *older*
ones — 0.6 / 0.35 / 0.05, oldest first. The newest pair, the one containing the
lift-off, carries a twentieth of the weight.

```rust
pub enum VelocityStrategy {
    Regression,
    RecentAverage([f32; 3]),
}
```

`VelocityTracker::platform_default()` picks at compile time from the target, exactly as
`ScrollPhysics::platform_default()` does — and the pairing is not a coincidence: the
platforms that bounce are the platforms that read flings this way.

### Speed is not enough: the fling gate

A finger that twitches fast over three pixels produces a large velocity and no
intention whatsoever. Requiring the gesture to have **covered ground** as well is what
separates a throw from a wobble:

```rust
fn fling_velocity(estimate: VelocityEstimate, slop: f32) -> (f32, f32)
```

Zero on any axis the gesture did not really travel along. The gate is **per axis**
because a scroll is two independent axes: a swipe running down the screen must not
fling sideways on the little horizontal wobble a thumb always adds. That is why
`VelocityEstimate` carries `offset` alongside `velocity` — the two answers are only
useful together.

### The slop a pointer deserves

The distance that says "this was a drag" was one constant, 8 px, for every pointer.
Two things were wrong with it.

- **8 px is too small for a finger.** A thumb covers a wide contact patch and rolls as
  it presses. The mature toolkits started at 8 too, and raised it to **18** after
  hearing that targets were too hard to hit — the press slid into a drag before the tap
  registered. `TOUCH_SLOP` is now 18, and so is the long-press rejection threshold: a
  thumb held still for half a second still wanders several pixels.
- **18 px is far too large for a mouse.** A mouse knows exactly where it is, so almost
  any movement is deliberate. `PRECISE_SLOP` is **1 px**, and `hit_slop()` picks between
  them from the pointer that started the gesture.

The same slop gates the fling, so the two stay consistent by construction: whatever
distance was enough to call it a drag is enough to call it a fling.

## What it replaced

| | Before | After |
|---|---|---|
| Estimate | `v/2 + instant/2`, in three places | one tracker, fitted or platform-weighted |
| History | the last two samples | up to 20, within a 100 ms horizon |
| Resting finger | read as its last motion | read as stopped |
| Fling test | velocity only | velocity **and** distance, per axis |
| Drag slop | 8 px for everything | 18 px finger / 1 px mouse |
| Back gesture | fractions/s, smoothed inline | px/s from the tracker, divided by the width |

Three `Drag` variants lose their `velocity` and `last_t` fields. At most one drag is
active at a time, so the tracker and its clock live on the shell — which also means the
gesture's history and its clock now start together, in one place (`begin_gesture`),
instead of being reset field by field at each construction site.

## Verification

- `cargo test --workspace --exclude frus-gpu --exclude frus-test` — **657 tests, 0
  failures** (640 at milestone 277).
- `cargo build -p frus-hello --target wasm32-unknown-unknown` — OK.
- `cargo build --workspace --all-targets` — OK, no new warning.
- The demo runs, renders and exits cleanly.

The tests are where the confidence is:

- **the estimate** — a steady drag is read exactly; the lift-off gesture survives; a
  finger that stopped before letting go flings nothing; an old fast gesture on the far
  side of a pause does not leak into a slow new one; the ring buffer wraps without
  losing the motion; two samples give travel but no speed.
- **the fit** — a known quadratic is recovered to 1e-6; a degenerate system has no fit;
  confidence falls when the points do not follow the curve.
- **the gate** — a fast twitch that went nowhere is not a fling; the gate is per axis; a
  precise pointer needs almost no travel.

**What cannot be verified here:** whether a fling now *lands* where the thumb meant it
to. That is a device judgement, and the change is most visible in the case the old
estimate got worst — a long swipe released gently.

## What's left

- **The overscroll glow** — still nothing painted when a clamping fling reaches an end
  (milestone 277's leftover, unchanged).
- **The confidence is computed and ignored.** A low r-squared means a gesture the fit
  does not describe well; a caller could fall back or damp the fling. Nothing does.
- **No per-pointer trackers.** One tracker for the one active drag, which is right today
  because frus routes a single pointer. Multi-touch would want one per pointer id.
- **The estimate is never clamped by the physics' `max_fling_velocity` at the gesture
  layer** — `ScrollPhysics::ballistic` still clamps per axis further down. Correct, but
  it means the pan fling (which does not go through the physics) has no cap at all.
