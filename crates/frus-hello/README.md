# frus-hello

The smallest complete frus application, a counter, and the canonical place to start.

**Layer:** Application. It depends on the `frus` facade alone, and it is the source of the
template `frus create` writes, in
[`crates/frus-cli/template`](https://github.com/KalybosPro/frus/tree/master/crates/frus-cli/template): a change
to one belongs in the other. See
[ARCHITECTURE.md](https://github.com/KalybosPro/frus/blob/master/ARCHITECTURE.md).

## What to read

- `src/lib.rs`: a `StatefulWidget` and its `State` — a plain struct, handlers that are closures, a
  timer asked for by the build that wants it — then `frus::main!`.
- `src/bin/frus-hello.rs`: the thin desktop binary that calls the generated `run()`.
- `res/values/styles.xml`: the Android launch theme.

## Running it

```sh
cargo run -p frus-hello                    # desktop
cargo apk run -p frus-hello --lib          # Android, with cargo-apk, the SDK and the NDK
```

The web build is described in
[`web/README.md`](https://github.com/KalybosPro/frus/blob/master/crates/frus-hello/web/README.md).

## Part of frus

An example, not a library to depend on. See the
[workspace README](https://github.com/KalybosPro/frus#readme). Licensed under MIT or
Apache-2.0.
