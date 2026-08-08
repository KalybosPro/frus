# Milestone 95 — Implicit animations: per-widget curve & duration

## Analysis

frus already animated widget values **implicitly** (`Widget::anim_target` →
`Runtime::values`/`advance_values`): a widget declares a `[0,1]` **target
progress**, the runtime drives the retained value towards it (snapping on mount),
and each widget interpolates its properties by that progress at paint time
(switch, drawer, sheet…). Two gaps compared with the established
`Animated*`-with-`duration`+`curve` shape:

1. **Linear progress**: `advance_values` advanced the value at a constant rate.
   The result is robotic — abrupt starts and stops.
2. **A frozen duration**: a global constant (`ANIM_DURATION = 0.12 s`), not
   adjustable per widget.

Yet `frus-core` has long provided a complete and tested [`Curve`] API (Linear,
Cubic/Bézier, CriticalSpring, Interval, Flipped, `ease_*`) — **unused** on the
widget side. This milestone plugs it in.

## Technical decisions

- **Two opt-in methods on the** [`Widget`] **trait**: `anim_duration() -> f32`
  (default: the standard duration) and `anim_curve() -> Curve` (default:
  `Linear`). The defaults **preserve exactly** the previous behaviour — no
  existing widget changes its feel without asking (and linear is the conventional
  default anyway). Forwarded by the transparent wrappers (`Box`, `Keyed`,
  `Responsive`).

- **A curved timeline** ([`ValueAnim`]). Each animated value now retains
  `{ current, from, to, elapsed }`. Each frame: `t = (elapsed/duration)` clamped,
  `current = lerp(from, to, curve(t))`. A **change of target rebases** the
  timeline from the current value (`from = current`, `elapsed = 0`): so resuming
  is clean and continuous even mid-flight (0→1→0), exactly the implicit-animation
  model. Mounting adopts the target with no transition.

- **`current` is the only value read at paint time** (`Runtime::value`/`value_or`),
  the timeline staying internal. A `set_value(id, v)` helper sets a value at rest
  (isolated renders and tests).

## Demonstration

`Switch` adopts `Curve::ease_in_out()`: the knob **accelerates then slows** rather
than sliding at a constant speed — the canonical animation of a toggle. Any other
widget can now set its curve and duration in one line.

## Tests

- `curve_shapes_the_value_timeline`: through a mock widget, at t=0.25 an *ease-in*
  is **behind** the linear progress (0.25) and an *ease-out* is **ahead**; the
  linear one is exactly `t`; all of them converge on the target.
- `shorter_duration_animates_faster`: at equal `dt`, a shorter duration is further
  along (t=0.5 vs 0.125).
- The existing tests (`value_snaps_on_mount_then_animates`, the drawer's and
  sheet's `anim_target_*`) stay green: the endpoints and the mount snap are
  unchanged; the mid-progress renders go through `set_value`.

## What's left

- Ready-made `Animated*` widgets (colour/size/padding interpolated as one piece
  through [`Tween`]), and an `AnimatedOpacity` resting on the layers (J92-94).
- Per-**property** curves and staggered windows (`Curve::Interval`) are already
  possible in core, to be exposed at widget level.
- Animations driven by **multiple** targets (today there is one scalar progress
  per widget).
