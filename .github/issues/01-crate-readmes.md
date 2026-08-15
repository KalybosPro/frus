title: Give every crate a README
labels: good first issue, documentation

Every one of the 15 crates in `crates/` is missing a `README.md`. On crates.io that
is the whole of a crate's front page, so this blocks
[#publish-to-crates-io](../../issues) as much as it costs us readers today.

### What to do

One short README per crate. There is no need to take all fifteen — **one crate is a
perfectly good pull request**, and several small ones are easier to review than one
big one.

Each should answer, in about a screenful:

- what the crate is for, in one sentence;
- where it sits in the four layers (see [ARCHITECTURE.md](../../blob/master/ARCHITECTURE.md));
- the two or three types someone will actually reach for;
- one short, compiling example where that makes sense;
- a line pointing back at the workspace README.

The crate-level rustdoc (`//!` at the top of `src/lib.rs`) is already good in most
crates — start by reading it, and do not simply duplicate it.

### Done when

`crates/<name>/README.md` exists, `readme = "README.md"` is set in that crate's
`Cargo.toml`, and `cargo doc` still passes.
