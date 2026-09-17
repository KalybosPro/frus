# Milestone 536 — `frus-image` denies a missing doc comment

Issue #14 asks for `#![warn(missing_docs)]` on every crate, one pull request per crate,
findings fixed, then the lint promoted to `#![deny(missing_docs)]` once a crate is clean. It
names three small crates to start with: `frus-image`, `frus-l10n`, `frus-layout`. This is the
first, `frus-image`.

## The decision

Add the lint as `warn` first, as the issue asks, to see what it finds.

- **Nothing.** `frus-image`'s public surface is four items: the crate's own doc comment,
  `DecodeError`, its `message()` method, and `decode()` — and all four already carry a doc
  comment, `decode()`'s with a working example. `cargo clippy -p frus-image --all-targets -- -D
  warnings` came back clean with the lint at `warn`.
- **Promoted straight to `deny`.** The issue's plan is warn-then-fix-then-deny; with nothing to
  fix, there was nothing between the two steps. `deny` is what stops the lint from being quietly
  reintroduced by a later change — `warn` alone would not have.
- **A drive-by:** the test helper `encode()`'s `.expect("encodage")` was the one French word
  left in the crate, in a crate whose own doc comments and public error messages are otherwise
  all English. Fixed to `.expect("encoding")`. Nothing else in the file needed it.

## Verification

- `cargo clippy -p frus-image --all-targets -- -D warnings` — clean, both with the lint at
  `warn` (to confirm no findings) and at `deny` (the shipped state).
- `cargo test -p frus-image` — 4 unit tests, 2 doctests, all passing, unchanged by this
  milestone.
- `cargo fmt --edition 2021 --check crates/frus-image/src/lib.rs` — clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc -p frus-image --no-deps --all-features` — clean.

No mutation testing: the change adds an attribute and renames one word in a test helper's panic
message: there is no branch or condition for a mutant to hide in.

## What is left

- **`frus-l10n` and `frus-layout`**, the other two crates the issue names to start with, each
  its own pull request, per the issue's own "one crate per pull request."
- **The other twelve crates** in the workspace, past the three named ones — not scoped by the
  issue, left for whoever continues the sweep.
