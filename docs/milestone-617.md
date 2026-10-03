# Milestone 617 — The Android scrollbar, under an Android theme

## Objective

The reference's scrollbar looks and behaves differently under a theme whose platform is
Android (`scrollbar.dart:331`): `_useAndroidScrollbar` is set from `theme.platform`, and it
changes five things. frus drew the desktop bar on every platform. The bar is rarely seen on
Android — the scroll behaviour draws none there (milestone 616) — but an application or a
scrollable that asks for one there should get the platform's.

## What was done

Under a theme whose platform is Android, unless the theme says otherwise
(`ScrollbarTheme`):

| | Android | elsewhere | reference |
|---|---|---|---|
| thickness | 4 | 8 | `:310`, `_kScrollbarThickness / 2` |
| margin from the edge | 0 | 2 | `:357` |
| ends | square | a pill | `:355`, no radius |
| at rest | an opaque grey: the theme's highlight made opaque, `#CCCCCC` dark, `#BCBCBC` light | `on_surface` at 30 % dark, 10 % light | `:240`, `:246`; `theme_data.dart:501` |
| pointer | does not wake it, hover it or drag it | does | `:218`, `enableGestures` |

The hovered and dragged colours are the same on both, as in the reference; at rest the
Android thumb warms from its grey towards the hovered colour, as `Color.lerp` does there.

- `ScrollbarTheme::interactive`, the reference's `ScrollbarThemeData.interactive`: unset,
  a bar is interactive except under an Android theme.
- `Scrollbar::interactive` records it; `Ui::scrollbar_at` and `Ui::scrollbar_near` answer
  only for interactive bars, so a pointer neither wakes nor grabs an Android one.
- The platform is the **theme's**, as the reference reads it, not the scroll behaviour's.

Nothing changes off Android: the colour is now computed as a colour warmed towards
another rather than a level, and the two are the same arithmetic. Every golden is
unchanged.

## Not in this milestone

The reference also switches checkbox, radio and pull-to-refresh by the theme's platform,
but only in their `.adaptive` constructors, where iOS and macOS get the Apple-style widget.
frus has no Apple-style widgets to switch to; porting them is its own work.

## Tests

- `an_android_theme_draws_the_android_bar`: the same page under an Android and a Linux
  theme, whatever the test runs on — thickness, position, ends, colour at rest, and
  whether a pointer can wake or take the bar.
- `an_android_bar_takes_the_theme_s_word`: a theme that makes the bar interactive, or
  colours it, is obeyed on Android too.
