# Milestone 558 — `frus-shell` denies a missing doc comment

Issue #14, continuing the sweep. `frus-shell` is the only platform-dependent layer: the window,
the event loop, the IME and clipboard bridges, the entry points. Almost all of it is private; the
public surface is `Application`, `Command`, `Subscription`, `RemoteData`, `FrusApp`, the `run*`
functions, the `net` module and `App`, the handler that drives it all.

`#![warn(missing_docs)]` found seven locations:

- `App`, the event handler exported from `app`, which had no doc comment at all. It now says what
  it owns and that the entry points build it, rather than an application.
- The six variants of `net::Method` (`Get`, `Post`, `Put`, `Delete`, `Patch`, `Head`), whose enum
  and `as_str` were documented but whose variants were not. Each says what the verb is for.

Promoted to `#![deny(missing_docs)]` once the list was empty.

The check ran on the host with every feature, and again on `wasm32-unknown-unknown` and
`aarch64-linux-android`: a lint that is not run on a target is a lint that finds its findings
there later, and this crate has code that only exists on each.

## Verification

- `cargo clippy -p frus-shell --all-targets [--all-features] -- -D warnings` — clean.
- `cargo clippy -p frus-shell --no-deps --target aarch64-linux-android -- -D warnings` — clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc -p frus-shell --no-deps --all-features` — clean.
- `cargo fmt -p frus-shell -- --check` — clean.

`cargo clippy --target wasm32-unknown-unknown` reports one `needless_return` at the end of a
`cfg(web)` block of `app.rs`. It was there before this step, no CI job lints that target, and it
has nothing to do with documentation, so it is left as it is.

## What is left

- **`frus-widgets`**, the last crate, its own pull request.
