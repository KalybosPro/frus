# Milestone 746 — Shadows fade the whole way out

## Objective

The widest shadows, those of a surface twelve or twenty-four high (the widget gallery's
Elevation demo shows both), ended on a rim instead of fading into the page. It showed on
desktop and on a phone, through WebGPU and through WebGL, so it was not a backend's: it was
in the one shader every shadow goes through.

## The cause

A shadow reaches the renderer as a rounded rectangle and a blur: the rectangle is the shape
grown by the blur, its corners grown as much (milestone 606). The fragment shader faded it
with a `smoothstep` running from the shape's edge less the blur, at full strength, to the
rectangle's edge, at nothing. So every shadow:

- **stopped one blur beyond its shape.** A blur radius stands for a gaussian of deviation
  `0.57735 × blur + 0.5`, which goes on three deviations out, about `1.73 × blur`. A
  twenty-four high surface's widest shadow (blur 46, spread 8) stopped 54 pixels out where
  its gaussian goes on to about 89: the outer third of the fade was never drawn.
- **ended on a curve of its own making.** A `smoothstep` is a cubic: its fade is close to a
  straight ramp that bends to nothing within a few pixels at each end. Across a 92-pixel fade
  that bend is a contour: the eye finds where the shadow stops, all round the surface, as a
  rounded-rectangle outline. With a narrow blur the outline lies close to the shape and hides
  in it; with a blur of 38 or 46 it stands well clear of the surface, a rim.

Measured on the old shader (white shadow on a black page, where blending in linear light makes
the faintest tail visible), going out from a 24-high surface's right edge: `… 16, 14, 12, 9,
7, 5, 4, 3, 2, 1, 0` and nothing from 52 pixels on — a ramp that is still falling 2–3 levels a
pixel ten pixels before it stops.

Nothing else was at fault: no blur is clamped (`for_elevation` caps the height at 24, not the
blur), the decoration and `paint_elevation` both state the rectangle and radii the scene
expects, the scene scales the blur with the geometry, and no layer or cull is sized by a
shadow's rectangle.

## What was done

The quad shader (`crates/frus-gpu/src/shaders/quad.wgsl`) now draws a shadow as what it
stands for: **its shape convolved with a gaussian**.

- **The shape** is the rectangle less the blur all round, its corners less the blur (never
  below nought), exactly what the scene's convention puts there. `Scene::shadow`'s contract is
  unchanged, so nothing that builds shadows changed.
- **The deviation** is `0.57735 × blur + 0.5`, the usual conversion from a blur radius.
- **The coverage** is computed analytically: across each row, a rounded box convolved with a
  gaussian is a difference of two error functions, with the corner's arc deciding where the
  row starts and ends; down the column, the rows are summed over four samples of the
  gaussian. This is the well-known analytic rounded-box shadow; the error function is
  Abramowitz and Stegun 7.1.27, good to 5·10⁻⁴.
- **The quad grows** in the vertex shader, for a blurred instance only, by
  `3σ − blur + 1` pixels on every side, so the fade is drawn as far as the gaussian reaches
  and ends with less than a fifth of a percent left. The rectangle's own coordinates — the
  centre the shape is placed from, the gradients' `uv` — are unchanged by the margin.
- **Blur styles** (milestone 645) keep their meaning, read against the shape's edge: `Solid`
  keeps the shape solid under the fade, `Outer` keeps what lies outside the shape, `Inner`
  what lies inside.
- The docs of `Scene::shadow` and `BoxShadow::bounds` say where the fade now ends: a little
  beyond the rectangle.

## Alternatives weighed

- **Keep the `smoothstep` and widen the quad.** Still a cubic with its own ending: the rim
  moves out and stays. A gaussian has no ending to see.
- **Blur in a pass** (render the shape to a texture, blur it with the filter pipeline). Exact
  for any shape, but a texture and two passes per shadow, where the analytic form costs a few
  `exp`s in the pixels the quad covers. Shadows are of rounded rectangles here, which is what
  the analytic form handles.
- **Change the scene's convention** so the rectangle reaches three deviations out. Every
  caller of `Scene::shadow`, in frus and in applications, would have had to change, for a
  fact only the renderer needs.

## How it was tested

- `crates/frus-test/tests/shadows.rs`, new
  `a_surface_twenty_four_high_fades_into_the_page_with_no_rim`: a 96×64 surface twenty-four
  high, its shadow in white on a black page, sampled pixel by pixel out from its right edge
  in the rendered image. The fade must only darken going out, still be there 66 pixels out
  (past the old end at 54), be gone 100 pixels out, never step by more than four levels
  between neighbours, and end on a pixel lit by two levels at most. It **fails on the old
  shader** (nothing past 52 pixels) and passes on the new one. The milestone-606 test of a
  height of one (a faint, tight shadow) passes unchanged, and half strength at the shape's
  edge still holds.
- A one-off render of the gallery's Elevation demo (the same `Wrap` of 96×64 surfaces, on the
  light theme's background) before and after: below the twenty-four high surface the shadow
  now reaches the page's colour 64 pixels down instead of 55, and dies away instead of ending
  on an outline.
- The golden suite, under WSL (lavapipe, CI's mesa). Fifteen images failed, every one for a
  shadow: `decoration_images` (the four blur styles, the largest change: the soft edges now
  fade instead of ending on an outline; `Solid`, `Outer` and `Inner` keep their meaning),
  `decoration_shapes_sides_shadows`, `light_outlines`, `search_bars`, `search_view`,
  `floating_action_buttons`, `snackbar_action`, `toast_host`, `popover_and_portal`,
  `dropdown_widget_options`, `disabled_inputs`, `range_slider`, `range_slider_labels`,
  `sliders_reference` and `sliders_right_to_left` (the thumbs' shadows); at most 8 levels
  apart outside `decoration_images`. Regenerating also rewrote eleven within their tolerance,
  all of them differing only where a shadow is (by 1 or 2 levels): `controls_toggles`,
  `dropdown_menu`, `dropdown_menu_filtering`, `expansion_panels`,
  `explicit_decoration_and_text`, `menu_bar_open`, `popup_menu_rich`, `small_indicators`,
  `stack_grid_list`, `table_column_menu`, `table_reorder_preview`. A first version of the
  change moved every gradient by a rounding error (the quads' `uv` recomputed for every
  rectangle); it now keeps the corner's own `uv` where there is no margin, and nothing but
  shadows moved.
- The milestone-606 test `a_shadow_fades_to_nothing_with_no_step_at_its_edge` asserted the
  old ending (nothing left at the rectangle's last pixel). It now asserts the new one: still
  fading, faintly, at the rectangle's edge, and the page's white three deviations out.

## Not done

- **Not looked at on a phone or in a browser.** The shader is the same WGSL on every backend,
  and the bug was the shader's, but the fix was checked offscreen only.
- **Corners of different radii.** The row integral uses the radius of the quadrant the pixel
  is in, as the hard-edged fill already did. Where two neighbouring corners differ a lot
  under a wide blur, the fade can show a faint seam along the box's middle line.
- **Strength.** Shadows blend in linear light, like every translucent colour in frus, so a
  shadow at 20 % black over white paints lighter than the same shadow blended in sRGB. That is
  a deliberate, framework-wide choice and is untouched here; it changes how strong a shadow
  looks, not where it ends.
- `Primitive::bounds` still reports a shadow's rectangle, not the slightly wider area the fade
  reaches; nothing sizes a layer or culls by it today.
