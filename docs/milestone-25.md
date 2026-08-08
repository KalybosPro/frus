# Milestone 25 — DPI / scale factor (HiDPI)

Rendering only accounted for **physical pixels**: on a HiDPI screen (scale 2.0)
the UI was twice too small. From now on **the UI world is in logical pixels**;
the scale applies only at the **boundaries**.

## Principle

```
scale = window.scale_factor()

Input (cursor, wheel, gesture)   : physical → logical   (÷ scale)
Layout / view / build_ui         : in LOGICAL units      (physical size ÷ scale)
Output (rendering)               : logical → physical    (ui.scene().scaled(scale))
```

`frus-gpu` stays **completely ignorant of DPI**: it receives a scene already in
physical pixels and draws as before. The surface and the GPU viewport stay
physical; **glyphon** receives physical sizes and positions → **crisp** text.

## Where the scale applies

- **`frus-core`**: `Scene::scaled(factor)` + `Primitive::scaled(factor)` (plus
  `Rect::scale`, `Point::scale`) — scales geometry, radius, border, blur, clip
  and **font size**; leaves colours and text untouched.
- **`frus-shell`**: `App.scale` (read through `window.scale_factor()`, updated on
  `ScaleFactorChanged`); the cursor and the wheel's `PixelDelta` converted to
  logical; `view`/`build_ui` receive the **logical** size; rendering sends
  `ui.scene().scaled(scale)`. `resize` and the surface stay **physical**.
- **Widgets / demo**: **unchanged** — they were already written in "logical px".

## Technical decision (alternatives)

- **Transforming the scene on output** rather than scaling the GPU viewport or
  the shader. The reason: text (glyphon) has its own resolution; a logical GPU
  viewport would leave the text at the wrong size. Scaling the scene unifies
  quads **and** text, and leaves `frus-gpu` unchanged. The cost: one copy/scale
  of the scene per frame (negligible).

## Tests

- `Scene::scaled`: geometry ×factor (rect, radius, border, position, font size),
  colours and strings preserved.
- Non-regression at **scale 1.0**: the WSL demo (scale 1.0) runs identically.

## Limits (v1)

- Real HiDPI cannot be validated interactively here (WSL reports 1.0); covered by
  the `scaled` unit tests plus review.
- `ScaleFactorChanged` updates the scale and redraws; the associated physical
  surface resize is still handled by the `Resized` event.
