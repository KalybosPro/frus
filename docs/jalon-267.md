# Jalon 267 — A **single** entry point, one entry per platform

## The goal

A developer should write **one** entry point — `fn main() => run(App())` — and have the toolchain wire
Android/iOS/Web underneath. frus, though, forced every app to write **three** conditional entry
functions (`run_desktop`, `android_main`, the wasm `start`) plus a thin binary — ~15 lines of platform
plumbing duplicated in every project. This milestone replaces them with **a single declaration**.

## The `frus_shell::main!` macro

Invoked **once** in the app's library, it generates the entry points for **every platform**, all
delegating to the **same** application:

```rust
frus_shell::main!(App::default());
```

generates (conditionally on the target):

- **desktop** — `pub fn run() -> anyhow::Result<()>` (which the app's thin binary calls);
- **Android** — `#[no_mangle] fn android_main(AndroidApp)` (the native symbol the activity expects);
- **Web** — `#[wasm_bindgen(start)] pub fn start()`.

The argument is an **expression** that constructs the application (re-evaluated per platform, never
shared). The app writes **nothing** platform-specific any more.

## Details

- **`frus-shell/src/lib.rs`**: the `main!` macro (`#[macro_export]`) + the
  `#[doc(hidden)] pub use anyhow; pub use log;` re-exports so it is **self-sufficient** (the app does not
  have to declare those crates). The Web entry refers to `::wasm_bindgen` (the app's `wasm32`
  dependency, as an app keeps its UI toolkit in its manifest).
- **The thin binary** (desktop): `fn main() -> frus_shell::anyhow::Result<()> { <crate>::run() }` — a
  skeleton the **template** supplies and that is never edited (the equivalent of generated runners). A
  binary is still required because, cargo-side, `cargo run` targets a `[[bin]]` while `cargo apk`
  compiles the `cdylib` (`--lib`): the two targets are distinct, but the code the **developer writes**
  is single.
- **Migrated** to the macro: `frus-hello` (the canonical example), the `templates/app` **template**,
  `frus-demo` and `frus-transforms`. `frus-hello`'s and the template's `Cargo.toml` lose `anyhow` /
  `log` (re-exported by the macro); the template gains the `wasm-bindgen` dependency (targeted at
  `wasm32`) that the Web entry needs.

## Verification

- **Desktop**: `frus-hello`, `frus-demo`, `frus-transforms` compile (their binaries calling `run()`);
  tests `frus-hello` 2, `frus-widgets` 396, `frus-shell` 27, `frus-demo` 36 — all green.
- **Android**: the `frus-demo` APK built **and launched on a device** — the `android_main` the macro
  generates starts the whole app (logcat: the `stopwatch` subscription ticking each second; the "My
  Tasks" screen showing). So the native symbol is correct end to end.
- **Web**: the `#[wasm_bindgen(start)]` entry is generated (not built here; structurally identical to
  the old one, the `wasm-bindgen` dependency present).

## What's left

- One day, a **`frus` facade crate** re-exporting `frus-shell` + `frus-widgets`, so an app depends on a
  single `frus` and writes `frus::main!(...)` — closer still to a one-line setup.
