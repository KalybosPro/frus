# Milestone 525 — A front page for every crate

Issue #7: none of the fifteen crates in `crates/` had a `README.md`. On crates.io the README
*is* the crate's page, so a crate published without one says nothing about itself, and the
roadmap lists per-crate READMEs as one of the things standing between frus and publishing.
Today they cost readers too: a GitHub directory listing shows a crate's README under its
files, and every crate directory showed nothing.

## What was decided

**All fifteen, in one step, on one plan.** The issue allows one crate per pull request. The
value of fifteen short pages is that they read alike — someone who has read one knows where
to look in the next — and that is easier to get right written together than reviewed apart.

**One structure, two variants.** Every README has a title, one sentence saying what the crate
is for, its **layer** as `ARCHITECTURE.md` draws them, and a closing *Part of frus* line
pointing at the workspace README. Between the two:

- a **library** crate names the two or three things someone will reach for, and shows one
  short example;
- an **example** crate (`frus-hello`, `frus-demo`, `frus-transforms`,
  `frus-fetch-example`) says which files to read and the commands that run it;
- `frus-bench` says what each bench measures and how to run one.

`frus-bench` is in none of the four layers, and says so; `frus-test` sits in the application
layer because that is where `ARCHITECTURE.md` puts it.

**Not the rustdoc again.** The crate-level `//!` docs are mostly good, and the issue asks not
to copy them. The README orients — where the crate sits, what to open first — and leaves the
explaining to the docs. Where a crate's rustdoc already had an example that fitted
(`frus-l10n`, `frus-image`), the README's is a shortened version of it.

**Every link is absolute.** crates.io renders a README without the repository around it, and
no manifest carries a `repository` key yet, so a relative link to `../../ARCHITECTURE.md`
would be a dead link on exactly the page this is for. The links point at
`github.com/KalybosPro/frus`, on `master`.

**The examples are compiled, so they cannot rot.** A README is not a source file, and an
example in one that nothing compiles is correct on the day it is written. Each of the ten
library crates now carries

```rust
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;
```

so `cargo test --doc` runs the README's example with the crate's other doctests. The item
exists only under `cfg(doctest)`: it is not in the crate's API, and `cargo doc` never sees it.
The examples that need a window or a GPU — `frus-shell`, the `frus` facade, `frus-test` — are
`no_run`: compiled, not run. The others run, and assert something true: a scene holds the
rectangle pushed into it, ten rectangles that do not overlap are one draw call, a row's second
child starts after the first and the gap.

**`readme = "README.md"` in every manifest**, as the issue's *done when* asks. Cargo would
find a file of that name in the package root without it; saying so costs a line, and a crate
whose README is renamed then fails loudly instead of publishing without one. The key is set
per crate rather than inherited from `[workspace.package]`, because an inherited path is
resolved against the workspace root and would name the workspace README.

**The workspace README stops asking for them.** Its *Where to start* table, and the French
one's, lose the row for this issue, as milestone 507's lost theirs.

## Verification

**The examples are among the doctests, not assumed to be.** `cargo test --doc` on the ten
library crates passes, and `-- --list` names `ReadmeDoctests` in every one of them — so the
attribute does what it was put there for, and each README's example is compiled with the
crate's own doctests.

**`cargo doc`**, as CI runs it — `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
--all-features` — passes. Run first per crate on default features, it stopped at `frus-shell`,
on two links to `Request::json_body` and `Request::send_json`: methods that exist only with the
`json` feature. Those links are older than this milestone, and CI always documents with every
feature, so nothing here broke them — but `cargo doc -p frus-shell` on its own fails.

**`cargo metadata --no-deps`** reports `readme: "README.md"` for all fifteen packages, each its
own. The reason the key is not inherited was checked rather than remembered: in a scratch
workspace, a member with `readme.workspace = true` reports `..\..\README.md`, the workspace
root's.

**Every path, command, feature and variable a README names was looked up**: `src/bin/shots.rs`
and its `shots` feature, `web/README.md`, the three benches and the functions `src/lib.rs`
shares with them, `FRUS_UPDATE_GOLDENS`, `RUST_LOG`, the facade's `icons-*` and `images`
features, and `cargo generate --path templates/app`.

`cargo fmt --all -- --check` passes.

## What is left

- The rest of the crates.io metadata the roadmap lists: `repository`, `keywords`,
  `categories`, `documentation`, a publish order, and versions on the path dependencies.
- The four example crates are not marked `publish = false`, as `frus-bench` is. Whether an
  example belongs on crates.io is a decision for the publishing milestone, not this one.
- `#![warn(missing_docs)]`, crate by crate, is still its own roadmap item.
