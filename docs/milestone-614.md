# Milestone 614 — Which platform, and whose conventions

## Objective

An application could not ask frus which platform it runs on. Each widget that behaves
differently on Apple's platforms — scroll physics, scrollbars, the overscroll indicator,
the velocity tracker, where a bar puts its title — tested `cfg!(target_os = …)` on its
own, at compile time. So nobody could show iOS behaviour on an Android phone, a test
could only ever see the platform it was compiled for, and a web build, built for no
operating system, always followed Android's conventions, even in Safari on a Mac.

The reference answers with a `TargetPlatform` enumeration, a `defaultTargetPlatform`
with a debug override, a `kIsWeb` constant, and a `platform` on the theme that the
adaptive widgets read. This milestone adds the same foundation. Moving the widgets onto it
is the next one.

## What was done

In `frus-core` (`platform.rs`), re-exported by `frus-widgets` and `frus`:

- **`TargetPlatform`**: `Android`, `Fuchsia`, `Ios`, `Linux`, `MacOs`, `Windows`, with
  `ALL`, `of_build()` (the platform the binary was built for), `from_browser(platform,
  user_agent, max_touch_points)`, the families `is_apple`, `is_mobile`, `is_desktop`,
  and `name()` / `Display` ("iOS", "macOS"…).
- **`default_target_platform()`**: the debug override if one is set, else the browser's
  operating system on the web, else the platform built for.
- **`set_debug_default_target_platform_override(Option<TargetPlatform>)`** and its reader.
  It panics in a release build, as the reference's throws.
- **`IS_WEB`**: a constant.
- **`Platform`**: facts about the machine, which nothing overrides:
  `operating_system()` (`std::env::consts::OS`), `is_android()`, `is_ios()`,
  `is_linux()`, `is_macos()`, `is_windows()`, `is_fuchsia()`, `is_web()`,
  `number_of_processors()`, `path_separator()`.

In `frus-widgets`: **`Theme::platform`**, starting at `default_target_platform()`, and
`Theme::with_platform`. `Theme::lerp` switches it at mid-fade, as the reference does,
instead of dropping it.

In `frus-shell`: the web entry point reads `navigator.platform`, `userAgent` and
`maxTouchPoints` before any theme is built, records the browser's platform, and logs it.

## The browser's operating system

The reference's rule, in order:

1. `platform` starts with "Mac": **iOS** if the screen takes more than two touch points
   (an iPad asking for desktop pages calls itself a Mac), else **macOS**;
2. `platform` contains "iphone", "ipad" or "ipod": **iOS**;
3. the user agent contains "Android": **Android** (its `platform` is a Linux, which is
   why this comes before the next rule);
4. `platform` starts with "Linux": **Linux**;
5. `platform` starts with "Win": **Windows**;
6. anything else: **Android**.

## Alternatives weighed

- **Compile-time only** (`const fn`), as the widgets do today. That is what made it
  impossible to preview another platform, and it is wrong on the web, where the
  conventions to follow depend on the browser rather than on the build.
- **Folding Fuchsia into Android**, since frus does not run on it yet. The reference's
  own comment argues against it: a platform folded into another can never be given
  behaviour of its own without breaking whoever relied on the folding.
- **An override that also works in release builds.** An application that ships following
  another platform's conventions should say so in its theme, where a reader of the code
  sees it.
- **`Platform` folded into `TargetPlatform`.** On the web they differ, and a question about
  the machine (how many processors, which path separator) is not a design choice a theme
  should be able to change.

## Tests

- `the_default_is_the_platform_built_for`, `the_debug_override_wins_until_it_is_cleared`
  (the one test that touches the process-wide switch, and it resets it),
  `a_browser_reports_the_system_it_runs_on` (what real browsers report, including an iPad
  calling itself a Mac and Chrome on Android calling itself Linux),
  `platforms_fall_into_families`.
- `a_theme_follows_the_default_platform_until_told`, `a_fade_keeps_a_platform_whole`.
