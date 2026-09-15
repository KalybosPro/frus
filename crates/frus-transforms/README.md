# frus-transforms

An animated, interactive showcase of frus's transforms: translation, scale, rotation about
a pivot, and their compositions, with `AspectRatio` and `FractionallySizedBox`.

**Layer:** Application. A button inside a rotated `Transform` still answers a click, which
is hit-testing through the inverse matrix, shown rather than claimed. See
[ARCHITECTURE.md](https://github.com/KalybosPro/frus/blob/master/ARCHITECTURE.md).

## What to read

- `src/lib.rs`: a small state, a pure `update`, a subscription that keeps time while the
  animation runs, and a `view` driven by a `Tween` and by a slider.

## Running it

```sh
cargo run -p frus-transforms
```

## Part of frus

An example, not a library to depend on. See the
[workspace README](https://github.com/KalybosPro/frus#readme). Licensed under MIT or
Apache-2.0.
