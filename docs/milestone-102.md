# Milestone 102 — `AnimatedContainer`: animated padding

## Analysis

The last "signature" property of the animated box to cover: the **inner
padding**. Like the size (J98) it is a **layout** property: the interpolated
padding has to enter **at layout time** (to reposition the content), not at paint
time. So it takes the same injection point — [`effective_style`], already shared
by `build_layout` and the relayout cache's signature, which guarantees their
consistency (the cache invalidates for as long as the padding moves).

## Technical decisions

- **A per-side timeline.** The runtime keeps a [`PaddingAnim`]
  `{ current, from, to, elapsed }` per node, tweened by `advance_paddings` on the
  **same model** as size/colour/radius. Interpolation **side by side** (`Insets` =
  4 paddings).

- **The target = the *effective* padding (content + border).**
  `Container::style()` already reserves the border's space in the layout padding.
  So that this reserve is not **lost** when `effective_style` replaces the padding
  with the animated value, the target (`Widget::anim_padding`) is the
  **effective** padding — extracted into a single `Container::effective_padding()`,
  the source both of `style()` and of the animated target. (The border being
  constant, interpolating the effective padding amounts to interpolating the
  content padding, reserve included.)

- **`Container::animated_padding(padding, duration, curve)`** (uniform). Nothing
  changes at paint time; it is the layout that moves. All of a box's animations
  (opacity/colour/size/radius/padding) share one `(duration, curve)`.

## Implementation

- `frus-widgets`: `Runtime` (`PaddingAnim`, `paddings`, `anim_padding`,
  `advance_paddings`, `lerp_insets`); the `anim_padding()` trait method +
  forwarders; `Container` (`padding_anim`, `effective_padding()`,
  `.animated_padding`, `anim_padding()` + the duration/curve chain);
  `ui::effective_style` also injects `style.padding`.
- `frus-shell`: `advance_paddings` in the animation loop.

## Tests

- `animated_padding_tweens_between_frames` (runtime): snapping on mount (0), a
  linear 0→20 tween (halfway ≈ 10 per side), forgetting a widget that has gone.
- `animated_padding_insets_the_child_at_layout` (layout): halfway through, the
  child's background is **offset by ~10** — proof that the interpolated padding
  really does enter at layout time (the runtime → `effective_style` → taffy →
  rects chain).
- `visible_border_reserves_layout_padding` (existing) stays green: the
  `effective_padding()` refactor preserves the border reserve.
- The whole suite green (widgets 193).

## Animated-box scorecard (complete)

| Property    | Path                         | Milestone |
|-------------|------------------------------|-----------|
| opacity     | layer (GPU)                  | J96       |
| colour      | paint/`Status`               | J97       |
| size        | layout/`effective_style`     | J98       |
| radius      | paint/`Status`               | J99       |
| **padding** | **layout/`effective_style`** | **J102**  |

All carried by the same curved timeline (J95), and also exposed through the named
`AnimatedContainer` widget (J100).

## What's left

- Outer `alignment`/`margin`, composite `decoration` (finer parity).
- Generic typed `Tween`s; a dedicated animation demo.
