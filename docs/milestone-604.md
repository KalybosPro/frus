# Milestone 604 — Logs in the console, with nothing to set up

A developer writing `log::info!` or `log::warn!` in a frus application should see it. On
desktop, they did not.

## What was there

The shell set a logger up per platform:

- **Desktop**: `env_logger::init()`, whose filter with no `RUST_LOG` lets through **errors
  only**. A developer's `info!` and `warn!` printed nothing. So did the framework's own
  warnings: the overflow warning of milestone 335 ("a box … is overflowed by … px"), written
  for exactly this reader, was silent unless they already knew to set `RUST_LOG`.
- **Android**: `android_logger` at `info`, under a generic tag. A **panic printed nothing**:
  Rust reports a panic on standard error, and on Android that goes nowhere. A crash left no
  message in logcat.
- **Web**: `console_log` at `info`, every level through the same console method.
- **iOS**: no logger.

## What it does now

Logging is set up in one place, `frus-shell`'s `logging` module, with the same default on every
platform:

- everything from **`info`** up;
- the application's **own crate** from **`debug`** up, in a debug build. The crate is read from
  the application's type (`std::any::type_name`), so the developer names nothing;
- the graphics and windowing crates (`wgpu*`, `naga`, `winit` and the like), which talk a great
  deal at `info`, from **`warn`** up.

Per platform:

- **Desktop**: the terminal, through `env_logger` with that default. **`RUST_LOG` still decides
  over it**: `RUST_LOG=trace` shows everything.
- **Android**: logcat, **tagged with the application's crate name** (`adb logcat -s my_app`),
  with the same filter. A **panic** is logged as an error with its place in the source before
  the usual report, so a crash is readable in logcat.
- **Web**: the browser's console, with **a console method per level**: an error is red and a
  warning yellow in the developer tools. A small filter in `RUST_LOG`'s syntax does the
  per-crate part, since the browser has no `RUST_LOG`. `console_log` is no longer needed. Panics
  were already reported there by `console_error_panic_hook`.
- **iOS**: still nothing. That shell is still being set up, and has its own list.

## The framework's own messages

- The **inspector's tree dump** and the **hot reload**'s messages went through `eprintln!`. They
  now go through `log`, so they reach logcat and the browser too.
- The overflow warning, now visible, turned out to have **runs of twenty spaces** in the middle
  of its sentence. Its line continuations had been lost when it was written. Four other strings
  had the same damage: two messages a developer reads (`ThemeBuilder` read before being built,
  a measured size that is not a whole number of pixels) and two test messages. All five are
  mended.

## For the developer

`frus::log` is re-exported, so an application needs no `log` dependency of its own, and it is
now documented where a developer looks for it:

```rust
frus::log::info!("signed in as {}", user);
frus::log::warn!("the cache is {}% full", used);
```

`CONTRIBUTING.md` and the fetch example's README say what is shown by default and how to see
more.

## Verification

- The default filter: an application's `debug` is shown in a debug build and not in a release
  one. A framework crate's `info` is shown and its `debug` is not. A `wgpu_core` `info` is not
  shown, and its `warn` is. A crate whose name merely starts like the application's is not the
  application.
- A filter with a part it cannot read keeps the parts it can.
- The crate an application's type lives in is found.
- Clippy passes on desktop, Android and the web, where each platform's logger is compiled.
