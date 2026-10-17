# widgets — the frus widget gallery

Every frus widget with a live example, and the code that builds it, side by side.

**Try it in the browser: <https://kalybospro.github.io/frus/>**, the Web build of this project, published
from `master` by [`.github/workflows/pages.yml`](../../.github/workflows/pages.yml).

## Run it

```sh
cargo run                                  # desktop: opens on the list of categories
cargo run -- /widget/button                # straight to one widget's demo
cargo run -- "/widget/button?tab=code"     # ... on its code tab
cargo apk run --lib                        # Android
```

Each demo page has two tabs: **Example**, the widget running, and **Code**, the file that builds it.
Swipe or click between them.

## How it is organised

```
src/
  lib.rs                      entry point: FrusApp::router(..)
  common/
    catalog.rs                the table of contents: every category, in order
    routers.rs  routes.rs     "/", "/category/:id", "/widget/:id"
    shared.rs                 section(..) / sections(..): the layout every demo uses
  models/demo.rs              Demo and Category
  features/
    home/                     the list of categories
    catalog/                  a category's widgets, and the demo page (Example / Code tabs)
    layout/ text/ buttons/ inputs/ display/ feedback/
    navigation/ scrolling/ animation/ painting/ interaction/ state/
      widgets/<widget>.rs     ONE FILE PER WIDGET — the whole demo
      widgets/mod.rs          the category's list of demos
```

**To learn a widget, open its file.** Each one exposes `pub(crate) fn example() -> Box<dyn Widget>`
and nothing else of interest; that file is exactly what the Code tab shows.

## Adding a demo

1. Write `src/features/<category>/widgets/<widget>.rs` with `pub(crate) fn example() -> Box<dyn Widget>`.
2. Register it in that folder's `mod.rs`:
   ```rust
   Demo::new("my-widget", "MyWidget", "What it is for.", include_str!("my_widget.rs"), my_widget::example)
   ```

Ids must be unique across the gallery; `cargo test` checks it.

## Web

Once: `rustup target add wasm32-unknown-unknown` and
`cargo install wasm-bindgen-cli --version 0.2.127` (the version must match `wasm-bindgen` in
`Cargo.lock`). Then build, generate the JavaScript glue into `web/pkg/`, and serve `web/`:

```sh
cargo build --lib --target wasm32-unknown-unknown --profile web-release
wasm-bindgen --target web --no-typescript --out-dir web/pkg target/wasm32-unknown-unknown/web-release/widgets.wasm
python -m http.server 8080 --directory web     # http://localhost:8080
```

On the Web the address is the location: `http://localhost:8080/#/widget/button` opens a demo.

frus draws through WebGPU, which browsers offer on `https` pages and on `localhost`.

## Building against the published crate

Outside the frus checkout, replace the path dependency with `frus = "0.3"`.
