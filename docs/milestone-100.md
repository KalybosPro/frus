# Milestone 100 — Named widgets: `Opacity`, `AnimatedOpacity`, `AnimatedContainer`

## Analysis

J96→99 made `Container` able to animate opacity, colour, size and radius, but
through stacked methods (`Container::new().animated_color(…).animated_size(…)`).
The conventional shape for these capabilities is a set of **named widgets** —
`Opacity`, `AnimatedOpacity`, `AnimatedContainer` — which are more readable and
more discoverable. This milestone adds them as **ergonomic sugar**, without
duplicating a single piece of logic.

## Technical decisions

- **Transparent wrappers over an internal `Container`.** Each named widget
  contains a configured [`Container`] and **delegates everything** to it (the
  [`crate::Keyed`] idiom). The internal `Container` **is** the animated node; the
  user's child stays a **separate** node — so the per-node animated value
  (opacity scalar, colour, size, radius) cannot collide with a child that is
  itself animated. The identities (`child_id`) stay aligned with the paint walk,
  so animations and layout behave identically.

- **Delegation by macro, in qualified syntax.** A `macro_rules!
  forward_to_container!` generates the `Widget` impl by delegating exactly the
  methods `Container` overrides. A subtlety: `Container` has **inherent** methods
  with the same names as the trait's (`on_click`, `repaint_boundary`… = the
  builders); so the trait is called as `Widget::…(&self.inner)` to resolve the
  ambiguity. `debug_name` is **not** delegated: the inspector shows the named
  widget's name (`AnimatedContainer`, not `Container`).

- **`AnimatedContainer`: a builder with a shared duration and curve.**
  `AnimatedContainer::new(duration, curve)` then `.color()/.size()/.radius()/`
  `.opacity()/.padding()/.child()` — every animated property inherits the same
  `(duration, curve)`, consistent with a box having a single timing pair.
  `Opacity::new(o, child)` and `AnimatedOpacity::new(o, dur, curve, child)` wrap a
  child directly.

## API

```rust
AnimatedContainer::new(0.3, Curve::ease_in_out())
    .color(theme.primary)
    .size(200.0, 100.0)
    .radius(12.0)
    .child(Text::new("hi"))

Opacity::new(0.5, child)
AnimatedOpacity::new(0.0, 0.2, Curve::ease_in(), child)
```

## Implementation

- `frus-widgets`: a new `animated.rs` (`Opacity`, `AnimatedOpacity`,
  `AnimatedContainer` + the delegation macro); re-exports in `lib.rs`. No new
  animation logic — pure sugar over `Container`'s capabilities (J96→99).

## Tests

- `animated_container_declares_all_targets`: colour/size/radius/opacity +
  duration + curve correctly exposed through the trait (so they are picked up by
  the `advance_*` calls).
- `opacity_wraps_child_as_a_group`: a fixed opacity group, the child a **separate
  node** (no collision).
- `animated_opacity_declares_a_group_target`: animated opacity + a clean
  `debug_name` (`"AnimatedOpacity"`).
- The whole suite green (widgets 191, +3).

## Scorecard

The animated-box story is now carried **end to end**: the capabilities (opacity
J96, colour J97, size J98, radius J99) and the **named API** (J100), on J95's
curved-timeline infrastructure.

## What's left

- Animated padding/margin (injection at layout time, like the size).
- Generic typed `Tween`s; **explicit** animations (a driven controller).
- Static `alignment`/`decoration` on `AnimatedContainer`.
