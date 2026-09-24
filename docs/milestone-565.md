# Milestone 565 — Ready to publish, and not published

## Objective

Issue #13: everything resolves through local `path` dependencies, so using frus means cloning it.
The issue itself says to split the job — metadata and `publish = false` first, a release script
second — and to talk to the maintainer before the first real `cargo publish`, since a name on
crates.io cannot be taken back. This is the first half. **Nothing is published.**

## What it does

- **Ten crates are publishable, five are not.** `frus`, `frus-core`, `frus-gpu`, `frus-image`,
  `frus-l10n`, `frus-layout`, `frus-shell`, `frus-test`, `frus-text` and `frus-widgets` go out.
  `frus-demo`, `frus-fetch-example`, `frus-hello` and `frus-transforms` are applications, and
  join `frus-bench` in `publish = false`. `frus-test` stays: an application's own tests are its
  reason to exist.
- **Metadata on every crate.** `repository` is inherited from the workspace; each published
  crate states `documentation`, five `keywords` and its `categories`, beside the `readme` that
  milestone 525 already gave it. The category slugs were checked against crates.io's own
  category endpoint, which `cargo publish --dry-run` does not do.
- **Every path dependency between published crates carries `version = "0.2"`**, the one thing
  `cargo publish` refuses outright. A caret requirement, not a pinned one: the crates version
  **together**, as the workspace already has them (`version.workspace = true`), so the numbers
  cannot drift apart, and only a minor bump has to touch these lines.
- **`scripts/publish-dry-run.sh`** runs `cargo publish --workspace --dry-run`, which (stable
  since Cargo 1.90, the MSRV) orders the crates by the dependency graph and builds each from its
  packaged tarball against those packaged before it. The order it found:
  `frus-core`, `frus-l10n`, `frus-image`, `frus-layout`, `frus-text`, `frus-gpu`, `frus-widgets`,
  `frus-shell`, `frus-test`, `frus` — `frus-core` first and `frus` last, as the issue guessed.
  The run passed, every upload stopped by the dry run.

## Decisions

**Version together, not apart.** The issue asks. Ten crates with independent numbers means a
compatibility matrix nobody can hold in their head, for a project with one maintainer and a
public surface still moving; one number is what the workspace already says.

**No publish step in the script.** It would be a two-line change and the wrong one to make
casually. The first publish is the maintainer's, by hand, once.

**Not in CI.** The dry run compiles the whole graph three times and took about 35 minutes here. A metadata mistake is caught by the next person to run the script before a
release, which is the moment it matters; a required job of that length is a cost on every pull
request for a failure that arrives once.

## Left

- **The names are free today** — all ten answered 404 from crates.io's API on 2026-09-24 — and
  may not be tomorrow. That is the argument for the first `cargo publish` sooner rather than later.
- **The `cargo generate` template** still asks for `{{frus_path}}`; it drops the question only
  when there is something to depend on by version.
- **Per-crate crate-level docs with a runnable example** (the docs.rs half of the roadmap's
  "docs.rs-quality rustdoc") are a separate item.
