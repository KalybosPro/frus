# hello_world

The smallest frus application, laid out the way a real one is: one page that says hello.

## Run it

```sh
cargo run                       # desktop
cargo apk run --lib             # Android (device connected, cargo-apk installed)
```

## What to read

| File | What it shows |
|---|---|
| `src/lib.rs` | the module list and the **single entry point**, `frus::main!(FrusApp::new(HomePage))` |
| `src/bin/hello_world.rs` | the thin desktop binary that calls the generated `run()` |
| `src/features/home/pages/homepage.rs` | a `StatelessWidget`: a `Scaffold` with an `AppBar` and a centred `Column` of `Text` |
| `src/common/colors.rs` | the palette, asked for by role |

A page with nothing to remember is a `StatelessWidget`: it only says what it builds. When a
page needs to remember something, see [`../counter`](../counter).

## Web

Once: `rustup target add wasm32-unknown-unknown` and
`cargo install wasm-bindgen-cli --version 0.2.127` (the version must match `wasm-bindgen` in
`Cargo.lock`). Then build, generate the JavaScript glue into `web/pkg/`, and serve `web/`:

```sh
cargo build --lib --target wasm32-unknown-unknown --profile web-release
wasm-bindgen --target web --no-typescript --out-dir web/pkg target/wasm32-unknown-unknown/web-release/hello_world.wasm
python -m http.server 8080 --directory web     # http://localhost:8080
```

frus draws through WebGPU, which browsers offer on `https` pages and on `localhost`.

## Building against the published crate

This project depends on the frus checkout it sits in (`frus = { path = "../../crates/frus" }`).
Copied anywhere else, depend on the published crate instead:

```toml
frus = "0.3"
```
