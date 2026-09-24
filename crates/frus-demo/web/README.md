# frus-demo — the Web target (wasm + WebGPU)

The demo runs in the browser through **WebGPU**, from the same code as the desktop and Android
builds: `frus_shell::main!` generates the Web entry point. The page it serves is what an
application made of a router looks like on the web:

- **Every screen has an address.** `#/settings`, `#/wizard`, `#/task/3`… The address bar shows
  where the demo is, the back and forward buttons move it, and a reload — or a link to
  `…/#/task/3` — comes back to the same screen, with home underneath.
- **The tasks are kept in the page's `localStorage`**, there being no disk to write to.

See the [`frus-hello` page](../../frus-hello/web/README.md) for the prerequisites (the wasm
target and a matching `wasm-bindgen-cli`) and for serving; the demo differs only in its names:

From the repository root. Each command is on one line, so the same text runs in a Unix shell and
in PowerShell (a trailing `\` continues a line in the first and is a syntax error in the second):

```sh
cargo build -p frus-demo --lib --target wasm32-unknown-unknown --profile web-release
wasm-bindgen --target web --no-typescript --out-dir crates/frus-demo/web/pkg target/wasm32-unknown-unknown/web-release/frus_demo.wasm
cd crates/frus-demo/web
python3 -m http.server 8080    # `python -m http.server 8080` on Windows
# → http://localhost:8080/#/settings
```

`wasm-bindgen` must be **exactly** the version the build resolved — `0.2.127` in this
repository's `Cargo.lock` — and refuses a `.wasm` from another one with a message that names
both. `cargo install wasm-bindgen-cli --version 0.2.127`.

`--lib` because the crate also carries a desktop binary. A browser with WebGPU, and `localhost`
or `https`, as for the counter.
