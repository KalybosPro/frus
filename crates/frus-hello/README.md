# frus-hello

The smallest complete frus application, a counter, and the canonical place to start.

**Layer:** Application. It depends on the `frus` facade alone, and it is the source of the
`cargo generate` template in
[`templates/app`](https://github.com/KalybosPro/frus/tree/master/templates/app): a change
to one belongs in the other. See
[ARCHITECTURE.md](https://github.com/KalybosPro/frus/blob/master/ARCHITECTURE.md).

## What to read

- `src/lib.rs`: the state, a pure `update`, a `subscription` and a `view`, then
  `frus::main!`.
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
