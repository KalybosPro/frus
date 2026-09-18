# Milestone 544 — `frus-transforms` denies a missing doc comment

Issue #14, continuing the sweep. An animated, interactive showcase of `Transform`,
`AspectRatio` and `FractionallySizedBox`, driven by a `Tween` and by the user. `Showcase`
is private, `star_path` and the two tuning constants (`FRAME_DT`, `CYCLE`) are private
free functions/constants, and `frus_shell::main!(Showcase::default())` generates the
only public item, `run()`.

`#![warn(missing_docs)]` found nothing — the macro-generated `run()` is not flagged,
and everything local was already documented. Straight to `#![deny(missing_docs)]`.
