# Milestone 131 — Slimming down the `.wasm`

## Analysis

The Web bundle from milestone 129 weighed ~7.9 MB of **raw** `.wasm` under the default
`--release` — which, once run through `wasm-bindgen` and served gzipped, is **~2.86 MB
downloaded**. That is the first byte a visitor sees: transfer size is the metric that
counts (not the raw size on disk). The default release profile optimises for **speed**
(`opt-level = 3`), not size, and keeps the panic unwinding tables — useless for a target
where a small download comes first.

The goal: cut the **downloaded** `.wasm` without degrading rendering or touching the
**native** release (which must stay tuned for speed).

## Technical decisions

- **A dedicated `web-release` profile.** Cargo cannot tune a profile *per target*, but it
  does allow **named profiles**. We add `[profile.web-release]` (inheriting from
  `release`), enabled explicitly with `--profile web-release`. The native release is never
  affected.
  - `opt-level = "z"` — code **size** first (vs `3` = speed).
  - `lto = true` — cross-crate inlining + aggressive dead-code pruning.
  - `codegen-units = 1` — a single codegen unit: better optimisation.
  - `panic = "abort"` — removes the panic **unwinding tables**, dead weight on the Web (a
    panic there goes to the console anyway).
  - `strip = true` — drops symbols and debuginfo.

- **`wasm-opt`: measured, not assumed.** The `wasm-opt -Oz` pass (binaryen) shrinks the
  **raw** `.wasm`, but with the old binaryen available here (v108) it **slightly inflates
  the gzip** (a reordering that compresses less well). Since gzip size is what gets
  downloaded, we do not hard-wire it into the build flow: the README documents it as an
  **optional** pass, to be adopted only with a recent binaryen and after measuring the
  gzip.

## Implementation

- `Cargo.toml` (workspace): the `[profile.web-release]` profile
  (`inherits = "release"`).
- `crates/frus-hello/web/README.md`: building through `--profile web-release` (and the
  `target/wasm32-unknown-unknown/web-release/…` path), a size table, the `wasm-opt`
  caveat, a reminder to serve the `.wasm` compressed.

## Verification

Measured through `wasm-bindgen --target web` then `gzip -9` (the size actually
downloaded):

| build                     | after `wasm-bindgen` | gzip (transfer) |
| ------------------------- | -------------------: | --------------: |
| `--release` (default)     |          6,662,015 B |     2,864,956 B |
| `--profile web-release`   |          5,644,007 B | **2,556,282 B** |

- **Transfer: −308,674 B ≈ −10.8%**; raw post-bindgen: −15.3%.
- **Native intact**: the profile is additive; `cargo test --workspace` stays green, the
  native release keeps `opt-level = 3`.

## What's left

- Most of the remaining weight is **`wgpu` + `naga`** (the WebGPU driver) —
  incompressible without losing the rendering.
- Heavier levers, not taken here: `-Z build-std` + `panic_immediate_abort` (recompiles
  `std` optimised for size, but requires nightly); `wasm-opt -Oz` with a recent binaryen
  (an extra gain to be checked at the gzip level).
