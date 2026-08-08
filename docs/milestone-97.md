# Milestone 97 — `AnimatedContainer`: animated background colour

## Analysis

J95 made implicit animations **curved & configurable**; J96 delivered animated
group opacity. The announced continuation: `Animated*`s interpolating **other
properties** (colour/size/padding), in the `AnimatedContainer` shape.

The background colour is the **flagship** property — and, unlike size and
padding, it is **layout-free**: interpolating it requires no integration into
layout (which is computed *before* the animation). So it is the first multi-channel
`Animated*`, clean and self-contained.

## Technical decisions

- **Retained per node, tweened per channel.** The runtime keeps a [`ColorAnim`]
  `{ current, from, to, elapsed }` per widget and drives it towards the target
  through `advance_colors`, on the **same model** as J95's scalar value (rebasing
  on a target change, snapping on mount, the widget's curve and duration). The
  interpolation is done **channel by channel** (`Color::lerp`).

- **Delivered at paint time through `Status`, keeping `Status: Copy`.** The status
  now carries `anim_color: Option<Color>` (`Color` is `Copy` — no `Vec`, so
  `Status` stays `Copy` and nothing breaks at `paint`'s call sites). The walk puts
  the interpolated colour there (`Runtime::anim_color(id)`).

- **`Container` API** (the J96 idiom): `.animated_color(color, duration, curve)`.
  The trait gains `Widget::anim_color() -> Option<Color>` (the target), tweened by
  the runtime. At paint time, an animated background **wins** over the
  hover/pressed interpolation (an animated colour is *the* colour). A box's
  opacity and colour **share** one `(duration, curve)` (for simplicity; the two
  are rarely animated together).

- **Shell wiring**: `advance_colors(tree, dt)` joins the per-frame advancement
  chain (alongside `advance_values`), so the fade progresses and keeps requesting
  frames for as long as it moves.

## Why not size/padding (yet)

Animating a **layout** property requires the interpolated value **at layout time**
(taffy reads `style()`), so the animation has to be injected *before* painting —
a deeper integration into `build_ui`. Colour, being purely pictorial, animates
without touching it. Size and padding: a dedicated milestone.

## Implementation

- `frus-widgets`: `Runtime` (`ColorAnim`, `colors`, `anim_color`,
  `advance_colors`); the `anim_color()` trait method + forwarders
  (`Box`/`Keyed`/`Responsive`); `Status::anim_color`; `Container.animated_color` +
  paint; `ui::full_status` delivers the colour.
- `frus-shell`: `advance_colors` in the animation loop.

## Tests

- `animated_color_tweens_between_frames` (runtime): snapping on mount (red), a
  linear red→blue tween (halfway ≈ `(0.5, 0, 0.5)`), ending at blue, forgetting a
  widget that has gone.
- `animated_color_paints_the_interpolated_color` (scene): after advancing to the
  halfway point, the **painted background rectangle** carries the interpolated
  colour (the runtime → `Status` → paint → scene chain).
- The existing suites green: the path is inert without `animated_color`.

## What's left

- Animated **layout** properties (size/padding/radius) through injection at layout
  time; generic typed `Tween`s.
- **Named** `Opacity`/`AnimatedOpacity`/`AnimatedContainer` widgets (sugar over
  `Container`).
