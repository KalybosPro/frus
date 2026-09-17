# Milestone 540 — `frus` denies a missing doc comment

Issue #14, continuing past the three crates it named to start with. This is the
facade crate — the one thing a real application actually depends on — so it is
mostly `pub use` re-exports of items `frus-widgets`, `frus-shell` and `frus-text`
already document; `missing_docs` credits a re-export against the definition it
points at, so none of those needed anything new.

`#![warn(missing_docs)]` first, to see what it would find: one thing.
`pub mod fonts { ... }` — the module that gathers `add_font`, `set_default_family`
and `set_monospace_family` for the "Shipping" story — had only a plain `//`
comment above it, not a doc comment, so the lint didn't see it as documented.
Turned that comment into `///` on the module itself; its wording already said what
the module was for, so nothing needed rewriting, only the comment marker.

`#![deny(missing_docs)]` once that was the only finding.
