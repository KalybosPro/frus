# Milestone 452 — An application configures itself the way the reference's does

The criticism that started this one was precise, and it was right:

> In the reference, a lot of things are wired on the app widget and reachable anywhere
> through the context. We have no context — so it should be reachable through `self`.
> Since we have `Application`, it must handle everything the app widget does.

Two halves of that were already true and one was not, so this note says which.

## What `Application` already carried

`theme`, `title`, `window_size`, `density`, `accessibility`, `scroll_physics`,
`scrollbars`, `localizations`, plus the lifecycle and gesture hooks. And the *reachable
anywhere* half exists too, without a context: the ambient-scope idiom — `MediaQuery::of()`,
`localizations::of()`, and the `&Theme` handed to `view` and to every `paint`.

## What it did not

**One theme.** The reference's app widget takes four (`theme`, `darkTheme`,
`highContrastTheme`, `highContrastDarkTheme`), picks between them by `themeMode` against
the platform's brightness, and **animates the change**. `Application` had `theme()` and
nothing else.

The platform's brightness has been reported since milestone 380 —
`MediaQuery::platform_brightness`, read from the window manager on desktop and from the
system settings on Android — and **nothing could act on it**, because there was no second
theme to switch to.

The proof that this was a real gap and not a missing convenience is in this repo. The
demonstration application held

```rust
pub(crate) theme_from: Option<Theme>,
pub(crate) theme_progress: f32,
```

in its own state, advanced the progress in `tick`, captured the outgoing theme in **two**
reducer arms, and interpolated in `theme()`. That is the framework's work — the reference's
`AnimatedTheme` (`app.dart:1057`) — done by hand, and every application would have had to
do it.

## The shape

Six defaults on the trait, and the framework does the rest:

```rust
fn dark_theme(&self) -> Option<Theme>              { None }
fn theme_mode(&self) -> ThemeMode                  { ThemeMode::System }
fn high_contrast_theme(&self) -> Option<Theme>     { None }
fn high_contrast_dark_theme(&self) -> Option<Theme>{ None }
fn theme_animation_duration(&self) -> f32          { 0.2 }   // kThemeAnimationDuration
fn theme_animation_curve(&self) -> Curve           { Curve::Linear }
```

Every one of them defaults to *what happened before*: an application with a single theme
resolves to it whatever the platform says, so nothing already written changes behaviour.

`ThemeMode` is the reference's enum (`app.dart:57`) and lives in `frus-widgets` beside
`Theme`, with `wants_dark(brightness)` on it — because an application showing a
*System / Light / Dark* setting needs the same answer to tick the right row.

## Two things resolve, and both are the framework's

**Which theme.** `Application::resolved_theme` is the reference's `_themeBuilder`
(`app.dart:995`) rung for rung, including the one that is easy to get wrong: dark **and**
high contrast with no high-contrast dark theme takes the plain **dark** one — being right
about the brightness matters more than being right about the contrast — and falls to the
high-contrast light theme only when there is no dark theme at all.

It is on the trait rather than in the shell for the reason the criticism named: there is no
context here, so anything outside the frame loop that needs the theme an application would
be showing has to be able to ask `self` for it. The repo's own picture renderer does,
having previously called `theme()` and got whatever the application had blended.

**How it crosses.** `ThemeFade` in the shell, advanced once a frame. It watches the
**resolved theme**, not `theme_mode` — so a new seed colour crosses exactly the way a
light/dark switch does, which is what the demonstration's palette cycle needs and what
watching the mode would have quietly broken.

Three details worth writing down:

- The **first** frame does not fade. An application opens on the theme it asked for rather
  than crossing from one that was never on screen.
- A theme that moved is a **rebuild** — the view is a pure function of `(state, theme,
  size)` — and a fade still running asks for another frame. Both go through the same
  `app_animating` the tick already fed.
- **Reduced motion ends the crossing at once.** The theme still changes; it stops moving.
  That is what the setting asks for, and what this framework does with every implicit
  animation it runs.

## The demonstration lost code

```
theme_from, theme_progress            → gone from the model
the fade branch in tick               → gone
two reducer arms capturing the theme  → two plain assignments
theme_of(app) reading app.light       → theme_of(app, dark)
```

`ToggleTheme` is now `app.light = !app.light;` and the crossing still happens. That is the
measure of this milestone: the application says *what*, and the framework does *how*.

## The tests

Six, all driving the shell's own path rather than a harness's:

- `a_platform_s_dark_mode_reaches_an_application_that_never_asked` — including a mode
  asking for dark with nothing dark to give, which falls back rather than failing.
- `the_rungs_of_a_high_contrast_interface` — all four combinations and both fallbacks.
- `the_framework_crosses_from_one_theme_to_the_next` — no fade on the first frame, a blend
  that is neither theme in the middle, and arrival exactly once.
- `a_new_palette_crosses_the_same_way_as_a_light_dark_switch` — the reason the resolved
  theme is what is watched.
- `stillness_and_a_zero_duration_switch_at_once`.
- `a_theme_can_be_asked_for_before_the_first_frame` — the shell reads the layout direction
  off a theme while handling a gesture, which can arrive before one has been resolved.

## Still not there

`builder`, `locale` / `supported_locales` with resolution, `shortcuts` / `actions`, and
`color`. And one the theme now makes reachable and this milestone did not take: a `Theme`
cannot say whether it is light or dark, so nothing can do what the reference does on the
line right after it resolves one — set the system overlay style, which is what keeps a
status bar's icons legible against it (`app.dart:1012`).
