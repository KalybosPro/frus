# Milestone 658 — A new application's tests live in `tests/`

## Objective

The owner's direction (2026-10-15): an application made with frus keeps its tests apart from
its code, in a test folder, as a Flutter project keeps them in `test/`. The application that
`frus create` writes kept its one test inline, in a `#[cfg(test)]` module at the bottom of
`src/lib.rs`, next to the counter it tested.

Cargo's folder for that is `tests/`. Each file there is built as its own crate against the
application as a library, so it tests what `src/` makes public, as another package would.
The template is already a library (`crate-type = ["rlib", "cdylib"]`), which is what lets
`tests/` reach it.

The framework's own crates keep their tests inline: they test private internals (a field's
geometry, a slider's palette) that only code inside the crate can see.

## What was done

- **`src/counter.rs`**: the counter widget and its state, moved out of `src/lib.rs`. The
  state's fields and rules are public, so the tests can reach them.
- **`src/lib.rs`**: the application's entry. It declares `pub mod counter`, re-exports
  `Counter` and `CounterState`, and holds the one `frus::main!`.
- **`tests/counter_test.rs`**: the tests, through the application's public API (`use
  <crate_name>::CounterState`):
  - counting is plain state;
  - the count does not go below zero;
  - automatic counting toggles.
- The template's file list in `frus-cli` has the two new files. The test that compares it with
  the directory keeps them from drifting apart.
- The template's README says where the tests are.

## Left

- A **widget tester** for applications, like the reference's `WidgetTester`: pump the
  application, tap `+`, expect `1`. The shell has a windowless driver, but it is hidden and
  not exported to applications; offering one is a milestone of its own.

## Tests

- `frus-cli`:
  - a created application's `tests/counter_test.rs` names the application's crate;
  - every template file is built in, the new ones included.
- CI's `frus create` job creates an application and runs that application's own checks:
  `cargo test` there now runs `tests/`.
- By hand, CI's `frus create` job: an application created from the template has `src/counter.rs`
  and `tests/counter_test.rs`; `cargo fmt --check` and `cargo clippy --all-targets -D warnings`
  are clean; `cargo test` runs `tests/counter_test.rs`, 3 passed.

Mutation testing: 2 mutants, both killed:
- the test file left out of the built-in list;
- the test file naming a fixed crate instead of the application's.
