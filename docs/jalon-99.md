# Jalon 99 — `AnimatedContainer`: animated corner radius

## Analysis

After the size (J98, layout) and the colour (J97, paint), the last "signature"
property of `AnimatedContainer`: the **corner radius**. Like colour, it is a
**pictorial** property (it does not affect layout), so it follows the same light
path as colour — delivered at paint time through `Status`, without touching layout
or its cache.

## Technical decisions

- **A per-corner timeline.** The runtime keeps a [`RadiusAnim`]
  `{ current, from, to, elapsed }` per node, tweened by `advance_radii` on the
  **same model** as colour and size (rebasing on a change, snapping on mount, the
  widget's curve and duration). The interpolation is done **corner by corner** (a
  `BorderRadius` = 4 radii).

- **Delivery at paint time, `Status` stays `Copy`.** `Status::anim_radius:
  Option<BorderRadius>` (`BorderRadius` is `Copy`) — like `anim_color`, with no
  `Vec`, so `Status` remains `Copy` and no `paint` call site breaks. The walk puts
  the interpolated radius there (`Runtime::anim_radius(id)`).

- **`Container` API**: `.animated_radius(radius, duration, curve)` — uniform
  through an `f32` or per corner through a [`BorderRadius`] (as with `.radius`).
  At paint time, an animated radius **wins** over the fixed radius. All of a box's
  animations (opacity/colour/size/radius) share one `(duration, curve)`.

## Implementation

- `frus-widgets`: `Runtime` (`RadiusAnim`, `radii`, `anim_radius`,
  `advance_radii`, `lerp_radius`); the `anim_radius()` trait method + forwarders
  (`Box`/`Keyed`/`Responsive`); `Status::anim_radius`;
  `Container.animated_radius` + paint; `ui::full_status` delivers the radius.
- `frus-shell`: `advance_radii` in the animation loop.

## Tests

- `animated_radius_tweens_between_frames` (runtime): snapping on mount (0), a
  linear 0→20 tween (halfway ≈ 10 per corner), forgetting a widget that has gone.
- `animated_radius_paints_the_interpolated_radius` (scene): halfway through, the
  **painted background rectangle** carries the interpolated radius (~10) — the
  runtime → `Status` → paint → scene chain.
- The existing suites green: the path is inert without `animated_radius`.

## `AnimatedContainer` scorecard

The four "signature" properties are now animatable on `Container`, through the
same curved-timeline infrastructure (J95):

| Property | Path                     | Milestone |
|----------|--------------------------|-----------|
| opacity  | layer (GPU)              | J96       |
| colour   | paint/`Status`           | J97       |
| size     | layout/`effective_style` | J98       |
| radius   | paint/`Status`           | J99       |

## What's left

- Animated padding/margin (injection at layout time, like the size).
- **Named** `AnimatedContainer`/`Opacity`/`AnimatedOpacity` widgets (sugar over
  `Container`).
- Generic typed `Tween`s; explicitly driven animations (a controller).
