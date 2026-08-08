# Milestone 217 — Charts: animated pulsing halo on hover

## Analysis

Hovering already highlights the targeted point (milestone 211) through a **static** accented marker.
**Animated** feedback (a pulsing halo) catches the eye better. It is the first use of a **continuous**
animation in the charts domain — the `continuous()` + `Status::time` brick (the `Spinner`'s).

## Technical decisions

- **Opt-in through `.animated(bool)`.** Off by default. When on, `continuous()` returns `true`
  (continuous repaint) and the hovered point emits a **halo**: a circle that grows (`PULSE_GROW`) and
  fades over a cycle (`PULSE_SPEED`), derived from `Status::time`. The halo is painted **under** the
  solid marker.

- **Reuses the existing infrastructure.** No new runtime plumbing: `continuous()` already drives
  continuous repainting (proven by `Spinner`), `Status::time` supplies the elapsed time. The halo only
  shows on hover (inside the tooltip block) and outside stacked mode (where an individual height is
  meaningless).

- **A controlled cost.** Continuous repainting is requested **only** if `.animated` is set — a static
  chart stays free.

## Implementation

- `frus-widgets/src/chart.rs`: the `animated` field + `.animated(bool)` on `LineChart`;
  `continuous()` returns `animated`; the pulsing halo in the tooltip block; the `PULSE_SPEED` /
  `PULSE_GROW` constants.

## Verification

- `animated_pulse_adds_a_halo_and_requests_continuous_repaint`: `continuous()` follows `animated`; on
  hover, the animated chart draws **one circle more** (the halo) than the static one. (A continuous
  animation on hover: not *goldenable* through `render_widget`; covered by this test.)

## A note on running it

The freshly compiled test binary was **blocked by Smart App Control** (os error 4551) when launched
natively — this machine's known gotcha. The tests were run through **WSL** (a Linux ELF, outside
SAC), the chart's logic being pure (no GPU required).

## What's left

- A **pulse on BarChart** (the hovered bar's outline) and above all the **"on arrival" pulse on a
  grid cell** (milestone 214): a one-shot triggered by the focus, which requires a transient
  animation primitive in the runtime (not yet available) rather than a permanent `continuous()`.
