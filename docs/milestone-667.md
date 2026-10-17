# Milestone 667 — `frus create .` in an application

## Objective

`frus create .` in a project refused it: "is not empty: frus create writes only into a new or
empty directory". The reference's own tool does the opposite, and developers rely on it: run in
an existing project, it writes back the files that are missing — a platform directory never
generated, or one deleted — and leaves the rest alone. forge (`frus-examples/forge`) showed why
it matters: it was written by hand before the template had `web/`, so it could not run on the
Web, and the one command that knows what an application needs would not help.

## What was done

- **In a directory with files already, only the missing files are written.** Each file of the
  template is written if nothing is at its path, and left alone otherwise. Nothing is ever
  overwritten, so the command can be run as often as one likes; a second run writes nothing.
- **In an application** (a `Cargo.toml` with a `[package]`), the files are the application's:
  - its name is the package's, and the crate's is its library's — the directory may be called
    anything, and `--name` that disagrees with the manifest is refused;
  - the organisation is the one its Android package already has;
  - its `Cargo.toml` is never written, nor the template's **sample code** (`src/lib.rs`,
    `src/counter.rs`, `tests/counter_test.rs`): a `counter_test.rs` beside code that has no
    counter would not even build;
  - no desktop binary is added when it has one (`src/main.rs` or anything in `src/bin/`): a
    second of the same name is a build error, of another name a second program;
  - no git repository is made: the application's version control is its own business.
- **In any other directory with files**, the whole template is written around them; sample code
  only when there is no `src/lib.rs` there.
- **A workspace root** (a `Cargo.toml` without `[package]`) is refused, with a word on why.
- The report says what came back, file by file, or that nothing was missing.

## Alternatives weighed

- **Overwrite with `--overwrite`**, as the reference offers. Left out: a template file the
  developer changed is theirs, and a flag that silently replaces it is one typo from losing
  work. Delete the file and run `frus create .` again for a fresh copy.
- **Merging the manifest** (adding the Android metadata or the web profile a hand-written
  `Cargo.toml` lacks). Not done: editing a developer's TOML without a TOML library risks more
  than it gives. The README written alongside says what each section is for.

## Verified

- Unit tests: a directory with other files is filled in without touching them; an application
  that lost its `web/`, an icon and its sample code gets back exactly the `web/` and the icon,
  with its manifest, code and organisation kept; a second run writes nothing; a library named
  apart from its package and a binary of its own are respected; a workspace is refused.
- On a copy of forge: `frus create .` recreated `web/index.html` (loading `pkg/forge.js`),
  `README.md` and the two workflows, and said "nothing to recreate" when run again. `frus build
  web` then built forge for the Web (5.9 MB) — the first time forge has had a web build.

## Left

- A hand-written manifest is not completed: an application without `[package.metadata.android]`
  still needs it added by hand to build an APK.
