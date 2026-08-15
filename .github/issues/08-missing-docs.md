title: Turn on `#![warn(missing_docs)]`, crate by crate
labels: help wanted, documentation

No crate sets `#![warn(missing_docs)]`, so nothing notices a new public item arriving
undocumented. The tree is in good shape — the rustdoc pass has been blocking since
milestone 298 and there are no broken links — but "documented" is not currently
enforced.

### What to do

One crate per pull request:

1. Add `#![warn(missing_docs)]` at the top of that crate's `src/lib.rs`.
2. Fix what it finds.
3. Once a crate is clean, promote it to `#![deny(missing_docs)]` so it cannot slip
   back.

Start with the small ones — `frus-image`, `frus-l10n`, `frus-layout` — to find out what
the real cost is before touching `frus-widgets`, which is ~80 modules.

### Please do not

Write `/// The width.` on `pub width: f32`. A doc comment that restates the name is
worse than none: it silences the lint and teaches the reader nothing. If an item is
genuinely self-evident and needs no prose, say what it is *for* or what it is measured
in, which is the part a name cannot carry.
