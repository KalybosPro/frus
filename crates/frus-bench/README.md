# frus-bench

frus's performance harness: what a frame costs on the CPU, measured with
[criterion](https://crates.io/crates/criterion).

**Layer:** none of the four. It is a measuring instrument that uses `frus-core`,
`frus-gpu`, `frus-text` and `frus-widgets`, and it is never published
(`publish = false`). See
[ARCHITECTURE.md](https://github.com/KalybosPro/frus/blob/master/ARCHITECTURE.md).

## What is here

- `src/lib.rs`: the screens every bench measures (`task_list`, `task_list_wordless`,
  `nested`) and `build`, kept in one place so that two runs measure the same trees.
- `benches/scene.rs`: building a widget tree into a scene, with and without text.
- `benches/text.rs` and `benches/batch.rs`: text measurement and batch planning.

## Running it

```sh
cargo bench -p frus-bench                  # every bench
cargo bench -p frus-bench --bench scene    # one of them
```

The numbers go to the terminal; there are no HTML reports.

## Part of frus

See the [workspace README](https://github.com/KalybosPro/frus#readme). Licensed under MIT
or Apache-2.0.
