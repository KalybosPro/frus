# Milestone 664 — The examples' tests live in `tests/`

## Objective

The owner's direction (2026-10-15): an application made with frus keeps its tests apart from
its code, in a folder of their own. Milestone 658 did it for the application `frus create`
writes. This milestone does it for the example projects in `frus-examples/`, which kept 158
tests in 60 `#[cfg(test)]` modules at the bottom of their source files.

Cargo's folder for that is `tests/`. Each file there is built as its own crate against the
project as a library, so it reaches only what `src/` makes public. The examples were already
libraries (`crate-type = ["rlib", "cdylib"]`); their modules and many of the items their tests
used were private.

## What was done

In each of the seven projects with tests (authentication, counter, forms, http, navigation,
websocket, widgets):

- **Each test module moved to `tests/<module>_test.rs`**. It is headed by the module it tests.
  - `use super::*` became the module's public path.
  - The module's own `use` lines came along, since a glob from outside does not carry them.
  - `crate::` and `super::` paths became the project's crate.
- **What the tests reach became public**:
  - the module declarations (`pub mod`);
  - the `pub(crate)` items;
  - the tested modules' top-level items;
  - in `widgets`, the 30 methods and fields its tests call, found from the compiler's errors
    and nothing more.
- **Test-only code became ordinary code**:
  - `Session::in_memory`, `Session::at` and `UserStore::outbox` were compiled only for the
    crate's own tests, so they are no longer gated;
  - the helpers the authentication tests shared (`answer_now`, `signed_out`) moved to
    `tests/support/mod.rs`, which each test file that needs them declares.
- Clippy's `new_without_default`, which public constructors now meet, is answered with a
  `Default` where it fits. `Auth::new` is allowed instead: it reads the session saved on the
  device, which a default should not.
- The comments that said "tested below" name the test file. The examples' README shows
  `tests/` in the projects' layout.

## Tests

- Every project: `cargo test`, `cargo clippy --all-targets -D warnings` and `cargo fmt --check`
  are clean.
- The tests moved, run in their new place:

  | project | tests |
  |---|---|
  | authentication | 36 |
  | counter | 2 |
  | forms | 21 |
  | http | 24 |
  | navigation | 27 |
  | websocket | 16, and 1 ignored as before |
  | widgets | 31 |

  That is 158, every one of them.
- websocket was checked with the Windows toolchain: WSL could not reach crates.io for
  `tungstenite`.
- No `#[cfg(test)]` is left in any project's `src/`.
