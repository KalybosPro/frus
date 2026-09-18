# Milestone 547 — `frus-text` denies a missing doc comment

Issue #14, continuing the sweep. Text measurement on top of `cosmic-text`, all in
one 1,886-line `lib.rs`, feature-gated bundled fonts included.

`#![warn(missing_docs)]` found nothing, checked with `--all-features` (every
`bundled-*` family compiled in) and with the plain default build — every public
item was already documented. Promoted to `#![deny(missing_docs)]`; the crate's own
38 unit tests still pass.

**Found while checking, not fixed, out of scope for this issue**: `cargo clippy -p
frus-text --no-default-features` fails on unrelated `dead_code` — `load_emoji_faces`,
`FontDir` and its two methods are only reachable through code gated on the bundled
font features, so with every one of them off the compiler correctly sees them as
unused. Pre-existing on `master` before this branch, and CI never compiles this
crate with `--no-default-features` (`.github/workflows/ci.yml`'s `clippy` job runs
plain `cargo clippy --workspace --all-targets`), so nothing here was ever green
under that configuration in the first place. Left as found — a `dead_code` finding
under a feature combination nothing exercises is a different problem from a missing
doc comment, and folding an unrelated fix into this branch would make the diff
harder to review for what it actually claims to do.
