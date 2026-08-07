# frus-hello — the Web target (wasm + WebGPU)

`frus-hello` runs in the browser through **WebGPU** without a single line of the app
changing: the **single entry point** `frus::main!(Counter::default())` generates the
Web entry `#[wasm_bindgen(start)]` itself — which calls `frus_shell::run_web` — exactly
as it generates the desktop and Android ones. The app writes **nothing**
platform-specific.

## Prerequisites (once)

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli   # a version matching wasm-bindgen 0.2
```

A browser with **WebGPU**: Chrome or Edge 113+, or Firefox with WebGPU enabled. WebGPU
requires a **secure context** (`https` or `localhost`).

## Building

```bash
# 1) Compile the library to wasm with the Web profile (optimised for SIZE, milestone
#    131: opt-level=z, lto, codegen-units=1, panic=abort, strip).
cargo build -p frus-hello --target wasm32-unknown-unknown --profile web-release

# 2) Generate the JS glue and the bound .wasm into web/pkg/.
wasm-bindgen \
  --target web --no-typescript \
  --out-dir crates/frus-hello/web/pkg \
  target/wasm32-unknown-unknown/web-release/frus_hello.wasm
```

### Size (milestone 131)

The `web-release` profile cuts the **downloaded** `.wasm` (gzipped) by about **11%**
against the default `--release`:

| build                     | after `wasm-bindgen` | gzip (over the wire) |
| ------------------------- | -------------------: | -------------------: |
| `--release` (default)     |            ~6.65 MiB |            ~2.86 MiB |
| `--profile web-release`   |            ~5.64 MiB |        **~2.56 MiB** |

Most of the remaining weight is `wgpu` plus `naga` — the WebGPU driver — which cannot be
compressed away without losing rendering. Always serve the `.wasm` **compressed**
(`Content-Encoding: gzip` or `br`); the gzipped size is what gets downloaded.

> An optional `wasm-opt -Oz` pass (binaryen) shrinks the raw `.wasm`, but **only helps
> the gzip with a recent binaryen** — an older version can reorder the code in a way
> that compresses *worse*. Measure the gzip before adopting it.

## Serving

A static server is enough; the ES modules and the `.wasm` must be served over HTTP, not
from `file://`:

```bash
cd crates/frus-hello/web && python3 -m http.server 8080
# → http://localhost:8080
```

## Notes

- **Rendering, input and animation** share the same code as desktop and Android; winit
  appends a `<canvas>` to `<body>`, the clock goes through `web-time`
  (`performance.now()`), and GPU init is **asynchronous** (see `run_web` and `resumed`).
- **Effects and subscriptions** are ported to the Web (milestone 130): `Command`s go
  through `spawn_local`, and `every` subscriptions through a browser `setInterval`,
  cancelled on drop. The counter's **Start auto** button starts an `every(1s)`
  subscription that increments it — the thing to watch in a browser. A genuinely
  **asynchronous** effect, a network fetch, is still to come; the current `Task` is
  synchronous.
- **Clipboard, accessibility and live reload** are disabled on the Web; each is a
  separate piece of work.
