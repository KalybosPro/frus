# Jalon 73 — Touch fling: scroll momentum (ballistics)

Scrolling's most visible gap (§6): letting go of the finger after a fast drag
**stopped the content dead** — there was no touch momentum (the wheel had its
spring inertia, the finger did not). This is the first meeting of milestone 53's
physics layer with scrolling.

## The mechanism (closed-form ballistics, the existing spring)

1. **Finger velocity**: during a `Drag::Scroll`, the velocity in scroll space
   (the content moves opposite the finger) is smoothed by an exponential moving
   average — the momentum at release.
2. **Ballistic projection**: `fling_destination(position, velocity)` = the
   **final position of a `FrictionSimulation`** (in closed form:
   `x∞ = x₀ + v/ln(1/drag)`, drag 0.135 — the usual constant: ~1000 px of travel
   for 2000 px/s). Below 50 px/s, no carry.
3. **Handover**: the destination (clamped with the existing elastic overshoot)
   becomes the **scroll spring's target**, seeded with the finger's momentum — a
   gentle deceleration, with the **bounce at the bounds for free** (milestone
   23's rubber-banding does the rest). A fling past the edge overshoots
   elastically and comes back — the native feel.

No new state and no new loop: `advance_scroll` is **untouched** (its pinned tests
pass as they are); the fling only *seeds* the target and the velocity. The wheel,
the bars and precise dragging are unchanged.

## Validation

- **250 tests**, all green — `fling_projects_a_friction_final_position` pins the
  closed form (≈ v/ln(1/drag), symmetry, the threshold), and the existing
  spring/bounce tests are unchanged.
- A warning-free build; the demo did not panic. The immediate beneficiaries:
  every finger-scrollable area — **Android** first (lists, the 5000-row Log).

## Not covered (accepted)

- Velocity by exponential moving average (the brief's **LSQ** fit = gesture stage
  3, deferred).
- The full restructuring into 4 pieces
  (`Position/Controller/Physics/Activity`) will come with the arena (stage 2) —
  in practice, this milestone delivers its `Physics::createBallisticSimulation`
  piece.

## What's left (remaining §6)

A regularised keyboard model (physical + logical + character), the
`padding`/`viewInsets` split (Android keyboard avoidance).
