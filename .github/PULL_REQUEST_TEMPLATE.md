<!--
Thanks for contributing to frus.

Keep the PR focused: one subsystem, one concern. A PR mixing a refactor with a
feature will likely be asked to split.
-->

## What this changes

<!-- One or two sentences. What does the codebase do now that it didn't before? -->

## Why

<!-- The problem this solves. Link the issue: Fixes #123 -->

## How

<!--
The approach, and — more importantly — the alternatives you considered and why
you rejected them. This is the part reviewers care about most.

If this is a milestone (new public API, a subsystem's shape changes, a
trade-off a future reader would question), add `docs/milestone-N.md` and say so here.
-->

## Testing

<!-- What you added, and what you ran. -->

- [ ] `cargo test --workspace` is green
- [ ] `cargo clippy --workspace --all-targets` adds no new warnings
- [ ] `cargo fmt --all -- --check` is clean **on the files I touched**

Platforms actually run on:

- [ ] Windows
- [ ] Linux
- [ ] macOS
- [ ] Android
- [ ] Web (wasm)

New tests:

- [ ] Logic tests (no GPU, no window)
- [ ] Golden tests — *I looked at every generated golden PNG by eye before committing it*
- [ ] Not applicable, because: <!-- explain -->

## Screenshots

<!-- Required for anything visual. Before/after if you changed existing rendering. -->

## Checklist

- [ ] Public items have `///` doc comments
- [ ] Any new styling is **overridable** — themed defaults, no hardcoded colors or dimensions in a widget's paint path
- [ ] No platform-specific code outside `frus-shell`
- [ ] New `unsafe`, if any, has a comment justifying why it is sound (or: none added)
- [ ] Any new gotcha I hit is documented in `CONTRIBUTING.md`

## Notes for the reviewer

<!-- Anything you're unsure about, deliberately left out, or want a second opinion on. -->
