title: Pin a minimum supported Rust version
labels: good first issue, build

No manifest carries a `rust-version`, and no minimum supported Rust version has ever
been tested. Development happens on whatever stable is current, so we do not actually
know what the floor is — only that it is *at most* current stable.

### What to do

1. Find the oldest stable toolchain that builds the workspace. Bisecting by hand with
   `rustup toolchain install 1.NN && cargo +1.NN check --workspace` is fine; so is
   `cargo-msrv` if you prefer.
2. Put `rust-version = "1.NN"` in `[workspace.package]` in the root `Cargo.toml`, and
   inherit it from every crate.
3. Add that toolchain to the CI matrix in `.github/workflows/ci.yml`, so a future
   change that raises the floor is a deliberate decision rather than an accident.
4. Say what it is in the README's prerequisites, which currently says the floor is
   unpinned.

### Notes

`wgpu`, `winit` and `cosmic-text` will dominate the answer. Do not raise the floor to
make a nicety compile — if a recent language feature is the only thing standing in the
way, say so in the pull request and we will decide together.
