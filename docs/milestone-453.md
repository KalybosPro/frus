# Milestone 453 — A theme could not say whether it was light or dark

Milestone 452 gave an application two themes and had the framework pick between them. It
ended on a gap it had made visible: **a theme, once picked, could not say which one it
was.**

The reference carries `brightness` on `ColorScheme` and on `ThemeData`. This carried it
nowhere, and one place in the crate needed the answer badly enough to work it out:

```rust
let dark = self.theme.scheme.surface.compute_luminance() < 0.5;
```

That is the scrollbar deciding how present its thumb should be. The reference reads
`_colorScheme.brightness` for exactly this (`scrollbar.dart:232`).

## Why a measurement is not an answer

The guess agrees with the declaration on all four schemes this crate builds — `dark()`,
`light()`, and `from_seed` either way — which is precisely why it survived every test.

It stops agreeing the moment an application writes a scheme of its own, which the public
fields invite. A **dimmed light** scheme — a reading theme, a low-light one — has a surface
below the halfway line and is not dark. The measurement says otherwise, and the numbers
either side of that question are three times apart at rest (0.10 against 0.30,
`scrollbar.dart:242`, `:248`). Getting it wrong is not a shade: it is a bar that reads as a
smudge, or one that reads as a stripe.

More generally, luminance is a property of one colour and brightness is a property of a
*design*. The two coincide often enough to be tempting and not often enough to be a rule.

## What was added

- `ColorScheme::brightness`, set by `dark()`, `light()` and both branches of `from_seed`.
- `Theme::brightness()`, reading the scheme — which is already the source of truth for the
  colours, so it is the source of truth for this.
- `Brightness::is_dark`, `is_light` and `inverted`. The last one is the question a surface
  asks about the content that has to be legible **on** it, and the next milestone needs it.
- The scrollbar reads it instead of measuring.

## It does not interpolate

`Theme::lerp` takes the **destination's** brightness, discretely, the way it already takes
the destination's text direction. Halfway between light and dark is not a third
brightness, and a widget reading one mid-crossing wants the answer it will still be right
about when the crossing ends — otherwise every fade would flip the scrollbar's opacities
at some arbitrary frame in the middle.

## Breaking

`ColorScheme` has one more field. Anything constructing one with every field listed must
name it; `..ColorScheme::light()` was already the usual way and is unaffected.

## What this unblocks, and what still blocks it

The line **after** the reference resolves a theme is
`SystemChrome.setSystemUIOverlayStyle(theme.brightness == dark ? light : dark)`
(`app.dart:1012`) — which is what keeps a status bar's icons legible against the surface
under them. Nothing here sets it, so a light theme on a device draws white icons on a white
bar.

The framework half of that is now writable. The platform half is not, yet, and the reason
is worth recording: setting the system bars' appearance on Android must happen on the Java
**UI thread**, and `android_main` is not it. The way across already exists —
`FrusTextBridge` does it with `runOnUiThread` — but adding a method to it means rebuilding
`assets/frus_input.dex`, which needs a JDK and `d8` from the Android build-tools. That is a
tooling prerequisite, not a design question, and it is recorded on the roadmap as one.

## The tests

- `a_scheme_says_whether_it_is_dark_rather_than_being_measured` — including the dimmed
  light scheme where the measurement and the declaration part company.
- `a_brightness_does_not_interpolate`.
- `a_bar_reads_the_scheme_s_brightness_and_not_its_luminance` — the same scheme, through a
  real frame, asserting the thumb's opacity.

The third fails with the luminance line put back — it is the one that reaches through a
frame to the pixel, and the only one that could. The first two are about the field itself,
which the guess did not remove so much as ignore.

**The goldens did not move**: the four schemes the pictures use are the four the guess was
right about.
