# frus-hello

The smallest complete frus application, a counter, and the canonical place to start.

**Layer:** Application. It depends on the `frus` facade alone, and it is the source of the
template `frus create` writes, in
[`crates/frus-cli/template`](https://github.com/frus-rs/frus/tree/master/crates/frus-cli/template): a change
to one belongs in the other. See
[ARCHITECTURE.md](https://github.com/frus-rs/frus/blob/master/ARCHITECTURE.md).

## What to read

- `src/lib.rs`: a `StatefulWidget` and its `State` — a plain struct, handlers that are closures, a
  timer asked for by the build that wants it — then `frus::main!`.
- `src/bin/frus-hello.rs`: the thin desktop binary that calls the generated `run()`.
- `res/values/styles.xml`: the Android launch theme.

## Running it

```sh
cd crates/frus-hello
frus run                 # desktop
frus run -d android      # Android: the connected phone, or an emulator
frus run -d web          # in the browser
```

`frus` comes from `cargo install frus-cli`. The web build by hand is described in
[`web/README.md`](https://github.com/frus-rs/frus/blob/master/crates/frus-hello/web/README.md).

## Part of frus

An example, not a library to depend on. See the
[workspace README](https://github.com/frus-rs/frus#readme). Licensed under MIT or
Apache-2.0.
