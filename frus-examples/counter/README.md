# counter

A page that remembers something: a `StatefulWidget`, its `State`, and the handlers that change it.

## Run it

```sh
cargo run                       # desktop
cargo apk run --lib             # Android
cargo test                      # the state's rules, with no window
```

## What to read

| File | What it shows |
|---|---|
| `src/features/counter/pages/counterpage.rs` | the widget (its configuration: the starting step) and `impl State` with `build` |
| `src/features/counter/states/counterstate.rs` | the state: a plain struct whose methods are the only way it changes — unit-tested |
| `src/features/counter/widgets/count_display.rs` | a piece of the page pulled out as a `StatelessWidget`, configured by its parent |

## The ideas

- **The widget is configuration, the state is memory.** `CounterPage` is made again on every
  rebuild; `CounterState` is created once by `create_state` and kept.
- **Handlers change the state, `build` only describes it.** `cx.callback(CounterState::increment)`
  turns a state method into a button's handler; `cx.handler(..)` does the same for a widget that
  reports a value (here, the `SegmentedButton`'s index).
- **Timers are asked for by the build.** While automatic mode is on, `build` calls
  `cx.use_interval(..)`; when it stops asking, the timer stops.
- **Rules are plain Rust.** Everything in `counterstate.rs` is testable without a GPU.

## Web

Once: `rustup target add wasm32-unknown-unknown` and
`cargo install wasm-bindgen-cli --version 0.2.127` (the version must match `wasm-bindgen` in
`Cargo.lock`). Then build, generate the JavaScript glue into `web/pkg/`, and serve `web/`:

```sh
cargo build --lib --target wasm32-unknown-unknown --profile web-release
wasm-bindgen --target web --no-typescript --out-dir web/pkg target/wasm32-unknown-unknown/web-release/counter.wasm
python -m http.server 8080 --directory web     # http://localhost:8080
```

frus draws through WebGPU, which browsers offer on `https` pages and on `localhost`.

## Building against the published crate

Outside the frus checkout, replace the path dependency with `frus = "0.3"`.
