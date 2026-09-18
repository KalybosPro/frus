# Milestone 538 — `frus-layout` denies a missing doc comment

The third and last of the three crates issue #14 names to start with (after `frus-image`,
milestone 536, and `frus-l10n`, milestone 537).

## The decision

Same plan: `#![warn(missing_docs)]` first, to see what it finds. Unlike the first two, this
one found something.

- **`Justify::Start`, `Center`, `End`, `SpaceBetween`, `SpaceAround`** (`style.rs`) had no doc
  comment — only `SpaceEvenly` did. Given one each, on the same plan as the sibling enum
  `AlignContent` a few lines above, which already documents its own `Start`/`Center`/`End`/
  `SpaceBetween`/`SpaceAround`.
- **`Align::Start`, `Center`, `End`** (`style.rs`) had none either — only `Stretch` and
  `Baseline` did. Given one each, again matching `AlignContent`'s phrasing.
- **`Layout::absolute_rects`** (`tree.rs`) had no doc comment of its own — and **`Layout::size_of`**,
  the function above it, had two doc comments run together as one: `absolute_rects`'s
  ("Walks the tree in prefix order…") followed straight by `size_of`'s own ("The size a node
  came out at…"), with no blank line or attribution between them, both landing on `size_of` and
  none on `absolute_rects`. `size_of`'s half already made sense on its own — it names
  `absolute_rects` as the whole-subtree counterpart — so this was a comment misplaced, not
  written twice: `absolute_rects`'s half is now above `absolute_rects`, where the code it
  describes (`taffy expresses positions relative to the parent…`) actually lives.

None of this touched a signature: every fix is a doc comment added or moved. The lint went
from `warn` to `deny` once clean.

## Verification

- `cargo clippy -p frus-layout --all-targets -- -D warnings` — clean, both with the lint at
  `warn` (after the fixes, to confirm nothing else was missing) and at `deny` (the shipped
  state).
- `cargo test -p frus-layout` — 13 unit tests, 1 doctest, all passing, unchanged by this
  milestone (no test exercises documentation).
- `cargo fmt --edition 2021 --check` on the three touched files — clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc -p frus-layout --no-deps --all-features` — clean.

No mutation testing: every change is a doc comment; there is no branch or condition for a
mutant to hide in.

## What is left

- **The other twelve crates** in the workspace, past the three issue #14 names to start
  with — not scoped by the issue, left for whoever continues the sweep.
