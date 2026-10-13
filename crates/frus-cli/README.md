# frus-cli

The command-line tool of [frus](https://github.com/KalybosPro/frus), a cross-platform UI
framework in Rust. It installs a program called `frus`, which starts new applications.

```sh
cargo install frus-cli
frus create my_app
cd my_app
cargo run
```

`frus create` writes an application that runs on Windows, macOS, Linux, Android and the web
from the same code: a counter built from a component, with its tests, the Android launcher
icons and launch theme, the web page, a CI workflow and a release workflow that publishes every
platform's build when you push a tag. Its `README.md` says how to run and ship it on each
platform.

```text
frus create <directory> [options]

      --name <name>        the package's name (default: the directory's name)
      --org <org>          the reverse domain the Android package starts with
                           (default: com.example; the package is <org>.<name>)
      --frus-path <dir>    build against a checkout of frus, not the published crates
      --no-git             do not make the directory a git repository
```

The directory is made, or must be empty: `frus create` never writes over a file. The name is
checked before anything is written. It must work as a package, a crate, a program, a `.wasm`
file and part of an Android package, so it is lowercase letters, digits, `_` and `-`, starting
with a letter, and neither a Rust keyword nor the name of a crate the application builds with.

The template is built into the program, so `frus create` needs no network. Its files are in
[`template/`](template), which is also a `cargo generate` template.

Everything after the first command is cargo's: `cargo run`, `cargo test`, `cargo apk run --lib`,
`cargo build --target wasm32-unknown-unknown`.

## License

MIT or Apache-2.0, at your option.
