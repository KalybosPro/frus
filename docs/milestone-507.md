# Milestone 507 — The oldest Rust that builds it: 1.88

Answers [#8](https://github.com/KalybosPro/frus/issues/8).

No manifest carried a `rust-version`, and no toolchain older than current stable had ever
built the workspace. The floor was known only to be *at most* current stable.

## The dependencies set it

The lockfile answers most of the question before anything is compiled: every crate in it
states the oldest compiler it supports, and the highest of those is a floor nothing in the
framework can go under.

| Asks for | Crates | Reached through |
|---|---|---|
| **1.88** | `icu_*` 2.3 (collections, locale core, normalizer, properties, provider) | `ureq` → `url` → `idna` — the HTTP helper in `frus-shell` |
| **1.88** | `image` 0.25.10 | `frus-image`, and `arboard` in the desktop shell |
| 1.87 | `zbus`, `zvariant` and their family | the Linux accessibility bridge |

So the floor is **at least 1.88** — and that it is not lower is shown, not inferred: on
1.87, cargo refuses the lockfile outright, naming the `icu` crates, before a single crate
is compiled.

## And the framework does not raise it

The other half is whether anything in the framework's own code needs a newer compiler
than its dependencies do. It does not: **1.88 checks the whole workspace, every target**
— the libraries, the demos, the tests and the benchmarks.

It warns twice, about a constant and a method in the icon blob's reader being unused. Both
are read, by a compile-time assertion that checks the bundled blob's signature; 1.88 does
not count a use there, and current stable does. That is a difference of opinion between
compilers, not a problem in the code, and it decides how the CI job is written.

## What is pinned

- `rust-version = "1.88"` in `[workspace.package]`, with a comment saying where the number
  comes from, and `rust-version.workspace = true` in all fifteen crates.
- A CI job, `msrv`, that runs `cargo check --workspace --all-targets` on 1.88.
  - **A check, not the test suite.** The tests run on stable in the jobs above; the
    question this job answers is whether the workspace still builds on the floor.
  - **Without `-D warnings`.** Lints move between releases, and the two above show why an
    older compiler's opinion is not the gate. Warnings are enforced by the clippy job, on
    stable.
- The prerequisites in both READMEs and in the contributing guide, which said the floor
  was unpinned.

Raising the floor is now **an edit of the manifest and of that job together** — and when a
dependency update raises it quietly, the `msrv` job is what says so.

## Not done, on purpose

The floor was not lowered by holding dependencies back. `image` and the `icu` crates at
older versions would build on an older compiler, but pinning old versions of a decoder and
of the Unicode data to buy compiler versions nobody has asked for is a trade to make when
someone asks, not before.
