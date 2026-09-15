# frus-demo

frus's larger sample application: a task list and a widget gallery that exercises most of
the framework, written as an outside consumer would write it.

**Layer:** Application. It uses `frus-shell`, `frus-widgets`, `frus-l10n` and `frus-image`
directly rather than through the facade. See
[ARCHITECTURE.md](https://github.com/KalybosPro/frus/blob/master/ARCHITECTURE.md).

## What to read

- `src/lib.rs`: the application, with the `android_main` the `cdylib` exports.
- `src/bin/frus-demo.rs`: the desktop entry point.
- `src/bin/shots.rs`: the tool that renders the pictures in the workspace README.

## Running it

```sh
cargo run -p frus-demo                     # desktop
cargo apk run -p frus-demo --lib           # Android, with cargo-apk, the SDK and the NDK
cargo run -p frus-demo --features shots --bin shots -- docs/media
```

## Part of frus

An example, not a library to depend on. See the
[workspace README](https://github.com/KalybosPro/frus#readme). Licensed under MIT or
Apache-2.0.
