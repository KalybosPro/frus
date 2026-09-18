# Milestone 545 — `frus-test` denies a missing doc comment

Issue #14, continuing the sweep. The framework's own test tooling: offscreen
rendering (`render_scene`, `render_widget`), the frame-loop harness (`Stage`) for
widgets whose picture is a gesture in flight, and golden comparison (`Snapshot`).

`#![warn(missing_docs)]` found one real gap: `Snapshot`'s three fields (`width`,
`height`, `rgba`) had a doc comment on the struct but none of their own. Following
the issue's own rule — say what a field is *for* or its unit, not its name back —
`width`/`height` are now documented as pixels and `rgba` as "four bytes per pixel,
row-major from the top-left corner." Everything else (`Stage`'s methods,
`assert_golden`, `write_png`, …) was already documented. Promoted to
`#![deny(missing_docs)]`; the crate's own unit test
(`a_real_screen_still_batches`) still passes.
