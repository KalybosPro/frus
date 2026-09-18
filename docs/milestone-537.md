# Milestone 537 — `frus-l10n` denies a missing doc comment

The second of the three crates issue #14 names to start with (after `frus-image`,
milestone 536): `#![warn(missing_docs)]`, findings fixed, then `#![deny(missing_docs)]`.

## The decision

Same shape as 536. Added the lint as `warn`, ran `cargo clippy -p frus-l10n --all-targets -- -D
warnings`: clean. Every public item — the crate doc comment, `Arg` and its three variants, the
`args!` macro, `Localizer` and its eight methods (`new`, `add`, `available`, `set_locale`,
`locale`, `get`, `format`, `format_for`, `langid`) — already carried a doc comment. Promoted
straight to `deny`; nothing to fix.

No drive-by this time: the file was already all English.

## Verification

- `cargo clippy -p frus-l10n --all-targets -- -D warnings` — clean.
- `cargo test -p frus-l10n` — 5 unit tests, 2 doctests, all passing, unchanged.
- `cargo fmt --edition 2021 --check crates/frus-l10n/src/lib.rs` — clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc -p frus-l10n --no-deps --all-features` — clean.

No mutation testing: an attribute with no findings to fix has no branch for a mutant to hide in.

## What is left

- **`frus-layout`**, the last of the three crates issue #14 names to start with, its own pull
  request.
- **The other twelve crates** in the workspace, not scoped by the issue.
