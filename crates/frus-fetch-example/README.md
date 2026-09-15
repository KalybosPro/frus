# frus-fetch-example

A networking example in one screen: a button starts a request, and the screen moves
through loading to data, or to an error.

**Layer:** Application. It depends on the `frus` facade alone, with the `net` feature. See
[ARCHITECTURE.md](https://github.com/KalybosPro/frus/blob/master/ARCHITECTURE.md).

## What to read

- `src/lib.rs`: `Command::perform_async` driving a future whose result becomes a message,
  and `frus::fetch` with a `Request` carrying headers and a timeout. `update` stays pure;
  the network lives in the `Command`.

## Running it

```sh
cargo run -p frus-fetch-example
RUST_LOG=info cargo run -p frus-fetch-example     # with logs
```

## Part of frus

An example, not a library to depend on. See the
[workspace README](https://github.com/KalybosPro/frus#readme). Licensed under MIT or
Apache-2.0.
