# Milestone 41 — sRGB / linear colour handling

Fixes a debt noted in `color.rs`: colours were being sent **as-is** (sRGB values)
to an **sRGB** surface. The GPU treats them as linear and re-encodes linear→sRGB
on write → **double encoding** → **washed-out** (too light) colours on screen.

## The fix

An sRGB target re-encodes linear→sRGB on output; so it has to be sent **linear**
values to reproduce the intended colour.

- **`frus-core`**: `Color::to_linear()` / `to_srgb()` (per-component conversion,
  alpha untouched), with the real sRGB curve (threshold 0.04045, exponent 2.4).
- **Quads** (`quad.wgsl`): the fragment converts the final colour sRGB→linear
  before writing it (`srgb_to_linear`).
- **Text** (`glyphon`): the colours are converted to linear before being passed
  to glyphon (same reason; the alpha stays as it is).

The scene's colours stay **authoring-friendly** (sRGB, like a colour picker); the
conversion happens at the very last moment, at the GPU boundary.

## Tests

- `frus-core`: fixed points (0→0, 1→1), the midpoint (`0.5` sRGB → `~0.214`
  linear), a `to_linear→to_srgb` round trip, alpha preserved.
- The offscreen renders (GPU tests) use **pure** colours (0/1), which are
  invariant under the conversion → still green.

## To be checked by eye (outside WSL)

WSL's software rendering does not let me **judge colours**. On a real screen the
colours should be **richer / more saturated** (less washed out), and the text
legible (neither too light nor too dark). The hypothesis: glyphon does not apply
the conversion itself — **to be confirmed visually**; if the text looks too dark,
remove the conversion on the text side.

## Limits (v1)

- **Gradients** are interpolated in sRGB space and then converted (a slightly
  different blend from one done in linear) — acceptable for v1.
- No wide colour space handling (P3, etc.).
