# Milestone 546 — `frus-demo` denies a missing doc comment

Issue #14, continuing the sweep. The sample to-do application: `lib.rs` declares
every module private (`mod model`, `mod screens`, `mod update`, …) except one,
`pub mod shots` (behind the `shots` feature), which already carried a doc comment.
A `pub` item inside a private module is not part of the crate's effective public
API and `missing_docs` does not see it — despite `frus-demo` being 7,500 lines
across twenty-odd files, its actual public surface is one function,
`shots::write_previews`, already documented.

`#![warn(missing_docs)]` found nothing, checked both with and without
`--all-features` (the `shots` module only compiles in with the feature on).
Straight to `#![deny(missing_docs)]`.
