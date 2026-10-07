# Milestone 631 — Visual density

## Objective

Reviewing the buttons against the reference (`docs/reference-review.md`) turned up something
the buttons depend on and frus did not have: **visual density**
(`material/theme_data.dart:3183`). In the reference every theme carries one. It is two
numbers from −4 to 4, and each step moves a control's base size by four logical pixels.
Unless told otherwise the theme takes it from its platform (`theme_data.dart:412`):
**standard** on Android, iOS and Fuchsia, **compact** (−2, −2) on Linux, macOS and Windows.
A desktop's buttons are therefore 8 px shorter than a phone's, and the same goes for its
checkboxes, radios, chips, icon buttons and list tiles.

frus drew every control at the phone's size on every platform. This milestone adds the
density, so that the widget reviews that follow can apply it.

## What was done

- **`VisualDensity`**: `horizontal` and `vertical`, kept within ±4; `STANDARD`,
  `COMFORTABLE` and `COMPACT`; `default_for_platform` (the reference's table);
  `base_size_adjustment` (4 px a step); `effective_min_size` (the reference's
  `effectiveConstraints`: a minimum size moved by the adjustment, kept between zero and the
  maximum); `lerp`.
- **`Theme::visual_density()`**: the density the theme was given
  (`with_visual_density`), else its platform's. Because it is read from the theme's
  platform, a theme set to Android shows phone-sized controls on a desktop, and the other
  way round, as in the reference. A density given explicitly stays when the platform
  changes. A fade moves between two densities; two themes that both follow their platform
  keep following it.

## What it changes on screen

Nothing yet. No widget reads the density in this milestone. Each widget's review applies
it, starting with the buttons, and says what moves.

## Tests

- `the_platforms_take_the_reference_s_densities`, `a_minimum_size_moves_by_four_a_step`.
- `a_theme_s_density_follows_its_platform`: platform defaults, a given density that
  outlives a platform change, and the fade.

Mutation testing: three mutants, all killed. They are the desktops at the standard density,
two pixels a step vertically, and a theme that ignores its platform.
