# Milestone 557 — `frus-gpu` denies a missing doc comment

Issue #14, continuing the sweep. `frus-gpu` is the crate that turns a `Scene` into pixels:
the device and queue, the batching, the pipelines, the text and image atlases, the offscreen
renderer the goldens are built on. Its modules are all private; what is public is a handful of
items re-exported at the crate root and the offscreen frame.

`#![warn(missing_docs)]` found two locations, both on `OffscreenFrame`: the `width` and `height`
fields, whose siblings `rgba` and `samples` were already documented. Each now says what it is
for and its unit — the frame's size in pixels — rather than restating its name.

Promoted to `#![deny(missing_docs)]` once the list was empty.

## Verification

- `cargo clippy -p frus-gpu --all-targets --all-features -- -D warnings` — clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc -p frus-gpu --no-deps --all-features` — clean.
- `cargo fmt -p frus-gpu -- --check` — clean.

No mutation testing: the change is two doc comments and an attribute, and has no branch for a
mutant to hide in.

## What is left

- **`frus-shell`** and **`frus-widgets`**, the two remaining crates, one pull request each.
