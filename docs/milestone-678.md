# Milestone 678 — `frus test` and `frus analyze`

## Objective

`cargo test` prints a line for every test and, for a failure, its whole output; clippy and
rustfmt are two commands, each with output of its own. A person wants to know what passed, what
failed and where, and what to fix: issue #290.

## What was done

- **`frus test [<filter>] [-v]`** runs `cargo test --no-fail-fast` in the application and says
  it briefly: the counts (`✓ Tests: 85 passed.` or `✗ Tests: 3 passed, 1 failed, 1 ignored.`),
  then each failed test with the place it panicked (`src/counter.rs:40:9`) and its message. A
  build that does not compile is said the same way, each error at its place. `-v` shows cargo's
  own output instead; the filter is cargo's.
- **`frus analyze [--fix]`** runs clippy on every target (`--message-format=json`) and
  `cargo fmt --check`, and lists everything together: each lint at its place with its code
  (`src/lib.rs:2:5: warning[clippy::needless_return]: …`), said once even when two targets
  lint the same file, then the files rustfmt would change. `--fix` formats the files and
  applies clippy's own suggestions first. clippy and rustfmt are installed (`rustup component
  add`) when the toolchain lacks them.
- Both exit with a failure while anything is left, for a CI step or a pre-commit hook.

The test harness's JSON output is not stable Rust, so `frus test` reads its text: the lines it
has printed the same way for years.

## Verified

- On a scratch package with a failing test, a needless `return` and an unformatted function:
  `frus test` gave the counts and the failure at `src\lib.rs:9:17` with its left and right;
  with an undefined name added, it said the error at its place; `frus analyze` listed the lint
  and the file; `frus analyze --fix` fixed both (outside a git repository too) and then found
  nothing.
- `cargo test -p frus-cli`: 91 tests, on Windows and on Linux (WSL).
