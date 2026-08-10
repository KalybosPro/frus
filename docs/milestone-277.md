# Milestone 277 — Scroll physics, per platform

## The goal

Scrolling worked; it did not *feel* like anywhere in particular.

One model served every target: a spring pulled the offset towards a target, a release
projected a friction endpoint, and an overscroll of at most 48 px was rubber-banded back —
identically on Android, on Windows and on the web. Two things were wrong with that.

- **It was a foreign feel everywhere.** A user's thumb is calibrated on the system's own
  scroll views. Android does not bounce; it stops dead at the edge, on a spline the platform
  publishes. The platforms that bounce do so with a specific resistance curve. Ours was
  neither.
- **The behaviour was not addressable.** There was no name for "how a scrollable behaves at
  its edges and after a fling", so there was nothing an application could set, and nothing a
  particular list could override.

This milestone gives that behaviour a name, two implementations, and a default that follows
the running platform.

## The shape

Three layers, each testable on its own:

```
frus-core   ClampingScrollSimulation, BouncingScrollSimulation   the maths
frus-widgets  ScrollPhysics { Bouncing, Clamping }               the policy
frus-shell    Application::scroll_physics()                      the choice
```

### The simulations (`frus-core`)

Both are `Simulation`s — pure `time → position` functions, `Copy`, no clock of their own —
joining the spring and friction already there.

`ClampingScrollSimulation` is Android's `OverScroller` fling: duration and distance derived
from the release velocity through the platform's constants, position following
`1 − (1 − t)^r`. It is adjusted to be **ballistic** — deceleration depends only on the
current velocity, never on how long ago the motion began — so a fling can be restarted from
its own mid-flight state and land in the same place. That property is a test:

```rust
let restarted = ClampingScrollSimulation::new(sim.x(0.15), sim.dx(0.15), tolerance);
assert!((restarted.final_x() - sim.final_x()).abs() < 1.0);
```

`BouncingScrollSimulation` is two motions end to end: friction while inside the content,
then a spring from the edge, seeded with the velocity friction had left at the hand-over.
The hand-over instant comes from inverting the friction curve — new on
`FrictionSimulation`:

```rust
pub fn time_at_x(&self, x: f32) -> f32   // ∞ when the motion never reaches x
```

Computing it once at construction is what keeps sampling a pure function of time; the
alternative, a mutable time offset updated as the simulation is read, would have made `x(t)`
depend on the order of calls.

### The policy (`frus-widgets/physics.rs`)

`ScrollPhysics` answers four questions, all as pure functions of `ScrollMetrics`
(`pixels`, `min`, `max`, `viewport`):

| | `Bouncing` | `Clamping` |
|---|---|---|
| `apply_user_offset` | resists past the edge, quadratically | passes the delta through |
| `apply_boundary_conditions` | refuses nothing | refuses whatever leaves the content |
| `ballistic` | friction → spring | platform spline, or a return to the edge |
| `carried_momentum` | repeated swipes accelerate | no carry |

The rubber band is the part worth stating plainly. Past an edge, the fraction of finger
movement that reaches the offset is `0.52 · (1 − out/viewport)²` — so the further out you
have pulled, the less each pixel of thumb buys. Easing *back* is measured at the position
the gesture is heading to, not the one it starts from, which is why releasing the band is
easier than loading it. That asymmetry is what makes a bounce feel elastic rather than
sticky, and it is a test of its own.

> **A sign convention worth writing down.** Our `delta` is a change *to the offset*; the
> source this behaviour was studied from passes an offset that is *subtracted* from the
> position. Every easing condition flips. The first implementation ported the conditions
> verbatim and had the band backwards — resistance fell off as you pulled further out. The
> unit test caught it, which is the argument for the policy being pure functions.

`platform_default()` resolves at **compile time** from the target: bouncing where the system
scroll views bounce, clamping elsewhere. A build is for one platform; a constant keeps the
choice out of the frame loop.

### The choice (`frus-shell`)

```rust
impl Application for MyApp {
    fn scroll_physics(&self) -> ScrollPhysics { ScrollPhysics::Bouncing }  // optional
}

Scroll::new().physics(ScrollPhysics::Bouncing)   // per area, wins over the app
List::new(5000, 44.0, row).physics(…)
```

An app that says nothing is native on each target. Both overrides exist because "which
platform am I" and "what does this particular list want" are different questions.

## Two motions, not one

The runtime now distinguishes them, and it matters:

- a **fling** — the finger let go with speed. The physics built a simulation; the runtime
  samples it. This is where the platform's feel lives.
- a **glide to a target** — the wheel, a programmatic scroll. The existing spring eases the
  offset across.

A fling wins while it runs: it drives the offset directly and keeps the target in step, so
the spring has nothing to pull against and nothing snaps back when the fling ends. How far
past the ends the *wheel* may push its target is now the physics' call too — the ±48 px of
elastic overshoot survives only where the platform bounces.

Under clamping physics the simulation knows nothing of the bounds, so the runtime clamps its
output and **ends that axis** the moment it hits an edge. Stopping dead is the behaviour, not
an approximation of it.

Two smaller consequences of the split:

- A finger back on moving content **catches** it, and hands the next release whatever
  momentum the platform lets a swipe build on (`carried_momentum`; zero where it does not).
- A release too slow to fling still asks the physics for a motion, because an offset left
  out of range has to come home regardless of how gently the finger left.

`crates/frus-shell/src/gesture.rs` loses `fling_destination` and its two constants: the
policy layer supersedes them.

## What it replaced

| | Before | After |
|---|---|---|
| Fling curve | one friction constant everywhere | the platform's own |
| Overscroll | ±48 px everywhere | none where the platform clamps |
| Where it lives | constants in the shell | a named policy, per app and per area |
| Registry entry | `(WidgetId, Rect, f32, f32)` | `Scrollable { id, viewport, max_x, max_y, physics }` |

`Ui::scroll_regions()` and `Ui::scroll_region(id)` replace the tuple-shaped
`scroll_hit`; `scrollable_maxes()` stays for the callers that only want the bounds.

## Verification

- `cargo test --workspace --exclude frus-gpu --exclude frus-test` — **640 tests, 0 failures**
  (613 at milestone 276).
- `cargo build -p frus-hello --target wasm32-unknown-unknown` — OK.
- `cargo build --workspace --all-targets` — OK, no new warning.
- The demo runs, renders and exits cleanly.

The new tests are where the confidence is, since the feel itself cannot be judged from a
software renderer:

- **`frus-core`** — the clamping fling covers the platform distance (~194 px at 1000 px/s)
  and is ballistic; the bouncing fling is continuous across the friction→spring hand-over,
  overshoots, and settles on the edge.
- **`frus-widgets/physics`** — the band resists more the further out you pull and less on the
  way back; clamping refuses exactly what would leave the content; a slow release flings
  nothing; the velocity cap holds.
- **`frus-widgets/runtime`** — a clamping fling stops *exactly* at the edge; a bouncing one
  overshoots and returns; an overscrolled offset comes home even with no fling; a fling on a
  region that left the frame is dropped.

**What cannot be verified here:** whether it feels right. The demo's log screen carries a
switch between the two behaviours for exactly that — fling to an end and compare, on a real
device.

## What's left

- **The overscroll glow.** A platform that clamps usually signals the edge with a glow
  instead of a bounce. Nothing is drawn today, so a clamping fling reaching an end is silent.
  That is a painting job, and it needs an eye on real hardware.
- **`ScrollDecelerationRate.fast`.** The second bouncing profile (a stiffer spring, a
  constant deceleration term) is not ported; only the normal rate exists.
- **The velocity estimate.** The release velocity is still an exponential smoothing of the
  last two samples, not a least-squares fit over a window. It is the input to everything
  above, so it is the next thing worth improving.
- **Keyboard and scrollbar scrolling** still go through the target-and-spring path, which is
  right — they are not gestures — but the physics has no say in them.
