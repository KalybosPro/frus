# Milestone 653 — `frus create`

## Objective

Starting an application took two tools and a path into this repository:

```sh
cargo install cargo-generate
cargo generate --path templates/app --name my-app
```

`--path` meant a clone of frus; `--git` meant knowing where the template lives. The reference
starts a project with one command, its own, and nothing to know beforehand. Now:

```sh
cargo install frus-cli
frus create my_app
```

## What was done

- **`frus-cli`**, a new crate whose binary is `frus`. No dependencies: it is compiled on the
  developer's machine by `cargo install`, and the standard library does everything it needs.
  The binary is `doc = false`, since its name is the facade's and both would write
  `target/doc/frus/`.
- **`frus create <directory>`**, with `--name`, `--org`, `--frus-path` and `--no-git`. Three
  steps, each testable alone: the names are checked, the files are rendered in memory, and only
  then is anything written. A directory the command made is removed again if a write fails;
  one that was there before is never touched unless it is empty.
- **The name** must work as a package, a crate, a program, a `.wasm` file and the last part of
  an Android package: lowercase ASCII letters, digits, `_` and `-`, starting with a letter, at
  most 64 characters. Refused, each with its reason: Rust keywords (the binary calls
  `my_app::run()`), the standard library's crates, cargo's own directory names, the frus
  crates and `wasm-bindgen` (with `-` and `_` treated alike, as cargo does), and Windows'
  device names. A name with capitals gets its lowercase form suggested.
- **`--org`**: the Android package becomes `<org>.<name>`, where the template used to write
  `com.example.<name>` and leave changing it to the README. Lowercase parts, none a Java
  keyword; the name itself is held to the same rule, as the package's last part.
- **`--frus-path`** replaces `frus = "0.3"` with a path to the checkout's `frus` crate. The
  facade reaches every other frus crate through its own path dependencies, so one line moves
  the whole framework, where the patch the README describes needs ten. The published
  requirement stays in a comment above it.
- **git**: `git init` unless `--no-git`, and not inside an existing repository, as `cargo new`.
  No git on the machine is a note, not a failure.
- **The template moved** into the crate, to `crates/frus-cli/template`. `cargo publish` only
  ships what is inside the crate, and a directory with a `Cargo.toml` in it is a separate
  package that it leaves out, so the manifest is now `Cargo.toml.liquid`; `frus create` and
  `cargo generate` both drop the suffix. The files are built in with `include_bytes!` from a
  list (`template.rs`) rather than a build script, which is one less program to compile and
  run on install.
- **Line endings**: a checkout on Windows has CRLF files, and `frus` built from it would have
  written them into every application. Text files are written with LF wherever the tool was
  built.

## What was found

- **cargo leaves dotfiles out of a package below its root.** With the default file list,
  `cargo package --list` had no `template/.gitignore` and no `template/.github/`: the published
  crate would not have compiled, since `include_bytes!` names them. The manifest now says
  `include = [...]` explicitly, and CI builds the tool from its tarball.
- **The template was built by nothing.** CI built `frus-hello`, the template's source, but no
  generated project: a template that stopped compiling would have been found by the first
  person to use it after a release. The new `frus create` job makes an application against the
  checkout and runs what its own CI runs — fmt, clippy with `-D warnings`, the tests, and the
  wasm build.

## Tests

`frus-cli`, 24 tests:

- names accepted and refused, each with its reason; organisations; the crate's name;
- rendering: every value filled in, no template syntax left in a path or a file, no `.liquid`
  suffix, LF only, `cargo-generate.toml` left out, the icons byte for byte, GitHub's `${{ }}`
  kept;
- `--frus-path`: the dependency replaced once, the published one kept in a comment, the path a
  valid TOML string on Windows (`\\?\` dropped, backslashes escaped);
- writing: a new directory, an empty one, a refused non-empty one with nothing touched, a file
  in the way, nothing on disk after a refused name or checkout;
- the command line: commands, defaults, both option spellings in any order, mistakes;
- the template in step with the crate: every file in `template/` built in and nothing else;
  the verbatim files the same as `cargo-generate.toml`'s excludes; the template's
  `frus = "x.y"` and `rust-version` those of this release.

By hand, on Linux (WSL) and Windows: `frus create` against the checkout; the generated
application's tests pass; a second `create` into it, a capitalised name and an unknown command
exit 1, 1 and 2 with their messages; on Windows the generated manifest's path is read by cargo.

**Not verified here** (the network was down during the milestone): the generated application's
wasm build and `cargo generate` on the moved template. The CI job covers the first.
