# Milestone 566 — On crates.io

## Objective

Milestone 565 made the ten library crates ready and published nothing, as issue #13 asks: a name
on crates.io cannot be taken back, so the first upload is the maintainer's decision. The
maintainer made it. This is the rest of the issue: the upload itself, then the template and the
words that said "not published".

## What happened

- **0.2.1, not 0.2.0.** The `v0.2.0` tag predates the web accessibility work and the publishing
  metadata; uploading "0.2.0" would have put on crates.io something the tag does not contain, under
  a number that cannot be rewritten. So the version moved to 0.2.1, was tagged, merged, and
  published from a **clean clone of the tag**, never from a working checkout — the main one carried
  a local workspace member that does not exist in the repository.
- **The rate limit.** crates.io lets a *new* crate through in a burst of five, then one every ten
  minutes (HTTP 429, with the time to retry after). The first five went out at once
  (`frus-core`, `frus-l10n`, `frus-image`, `frus-layout`, `frus-text`); the other five took about
  forty more minutes.
- **Cargo cannot resume `--workspace`.** Run again after a partial publish, it stops at the first
  crate that already exists ("already exists on crates.io index") rather than skipping it. The rest
  were published one at a time with `cargo publish -p <crate>`, in dependency order, waiting for
  each retry time: `frus-gpu`, `frus-widgets`, `frus-shell`, `frus-test`, `frus`. Cargo waits for
  each upload to reach the index before the next one starts, so the order held.
- **Verification.** All ten answer at 0.2.1 from crates.io's API. A project generated from the
  template was built against them, with no `path` anywhere.

## Decisions

**The template asks nothing.** The `{{frus_path}}` question existed only because there was
nothing to depend on by version. It is gone; `frus = "0.2"` is a caret requirement, so it follows
0.2.x. A developer who wants to try a change to frus from a generated project has a documented way
(`[patch.crates-io]` on all ten crates, since they version together and half a patch is a
combination nobody tests; the recipe was run against a checkout), which the guide now spells out.

**The old "Publish to crates.io" row leaves the contributor tables.** It was the README's
biggest "where to help" item; it is done, so it goes rather than stay as a false invitation.

**A crate's README is its crates.io page.** `crates/frus/README.md` still says "not on crates.io
yet" on the page of the crate that just went out. A published version cannot be edited; the fix
is in the tree and reaches the page with the next release. That is the price of writing "not
published" in a file that ships.

## Left

- The `frus` page on crates.io says "not on crates.io yet" until 0.2.2 (or whatever comes next).
- The GitHub release for 0.2.1 (and the still-draft 0.2.0) is the maintainer's to publish.
- Per-crate documentation with a runnable example, the docs.rs half of the roadmap's
  "docs.rs-quality rustdoc", is a separate item.
