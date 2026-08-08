# Jalon 57 — `BoxDecoration`: the box decoration model (§5)

With the engine foundations in place (the brief's Block A: phases, relayout
cache, animation physics), this milestone opens the **design system** (§5) with
its "paintable keystone": a **reusable decoration model**, where until now every
widget reinvented its own background/gradient/border/shadow by hand.

## What was missing

`Container::paint` composed its decoration inline (colour resolution,
`scene.shadow`, `scene.gradient_rect`/`draw_rect`), with no shareable type. No
`BoxDecoration`, no named paint primitive. Any widget wanting a decorated box
duplicated that logic.

## The core types (in `frus-core`, pure, `Copy`)

A new `decoration.rs` module:

- **`Border { width, color }`** — a uniform border (`is_visible`).
- **`LinearGradient { end, direction }`** — a gradient from the background
  towards `end`, anchored in `[0,1]²` space.
- **`BoxShadow { color, offset, blur, spread }`** — a soft shadow, with
  `bounds(rect)` (the offset/blurred/spread envelope).
- **`BoxDecoration { color?, gradient?, border?, radius, shadow? }`** — the
  complete decorated box, with:
  - **`paint_into(scene, rect, opacity)`**: lowers the decoration into `Scene`
    primitives in the **fixed order** shadow → background (solid or gradient) →
    border; `opacity` modulates every colour (the appearance fade).
  - **`content_padding()`**: the margin to reserve for the border — intended to
    feed taffy so that a bordered background does not eat into its content.

Also, the `Color` helpers the brief called for: **`with_alpha`**,
**`from_argb_u32`** (`0xAARRGGBB`), **`compute_luminance`** (WCAG, on linearised
channels — the basis of a contrast computation).

## Integration: `Container` adopts `BoxDecoration`

`Container::paint` now **composes** a `BoxDecoration` (colour resolved from the
hover/pressed state, gradient, border, shadow) and paints it through
`paint_into`. The inline painting logic disappears — replaced by the shared
model. The rendering is **strictly identical**: `frus-widgets`' 129 tests
(including the ones that inspect the produced primitives) and the demo's 15 pass
unchanged.

## Validation

- `frus-core`: **46 tests** (+9: the fixed paint order, `content_padding`, a
  border alone, opacity fading, shadow bounds; `with_alpha`/`from_argb_u32`/WCAG
  luminance).
- `frus-widgets` **129**, `frus-demo` **15**, everything else green — bit-for-bit
  identical output after refactoring `Container`.
- `cargo build --workspace` with no warnings.

## What's next (§5)

- **`content_padding` → taffy**: wiring the border reserve into the style so that
  bordered widgets size correctly (today the border is purely painted).
- **Per-corner radii** (a 4-corner `BorderRadius`) — this needs the SDF shader to
  evolve (a single radius today).
- **`Alignment`**, `EdgeInsetsDirectional::resolve(dir)` (RTL), radial/sweep
  `Gradient`, `TextStyle`/`TextSpan`, then the **structured theme** (M3 roles +
  type scale), and a baked-in state layer.
