# navigation

Moving between screens with the router: a small shop with sections, nested pages, a search
whose query lives in the address, a guarded checkout and a page for addresses that lead
nowhere.

## Run it

```sh
cargo run                                  # desktop, at /
cargo run -- /products/3                   # open it at any location
cargo run -- "/search?q=shoes&sort=price"  # query parameters too
cargo run -- /checkout                     # the guard sends an empty cart to /cart
cargo run -- /sale/2019                    # no route: the page-not-found page
cargo apk run --lib                        # Android
cargo test                                 # the cart, the guard, search and the page states
```

On the **web** you do not need the command line: the router's location *is* the address bar.
`https://host/app/#/products/3` opens that page with the shop underneath, moving between pages
changes the address and adds a history entry, the browser's back and forward buttons move the
router, and a reload comes back to the same page. `FrusApp::router(..)` does all of it.

## Web

Once: `rustup target add wasm32-unknown-unknown` and
`cargo install wasm-bindgen-cli --version 0.2.127` (the version must match `wasm-bindgen` in
`Cargo.lock`). Then build, generate the JavaScript glue into `web/pkg/`, and serve `web/`:

```sh
cargo build --lib --target wasm32-unknown-unknown --profile web-release
wasm-bindgen --target web --no-typescript --out-dir web/pkg target/wasm32-unknown-unknown/web-release/navigation.wasm
python -m http.server 8080 --directory web     # http://localhost:8080
```

frus draws through WebGPU, which browsers offer on `https` pages and on `localhost`.

On the Web the router's location **is** the address: `http://localhost:8080/#/products/3` opens
that page with the shop beneath it, and the browser's back and forward buttons move the router.


## The map

| Location | Page | How it is reached |
|---|---|---|
| `/` | shop front | a section: `go` |
| `/search?q=&sort=` | search | a section: `go`; typing and sorting `go_named` with the new query |
| `/cart` | cart | a section: `go` |
| `/products/:id` | product | `go_named` from the shop, `push_named` from search, `replace` for the next one |
| `/products/:id/reviews` | reviews | `push_named` |
| `/products/:id/size` | pick a size | `push_with_extra`, the product as `extra`; it pops with a result |
| `/checkout` | checkout | `push` from the cart; **guarded** by a redirect |
| `/order-placed` | confirmation | `go_with_extra`, the order as `extra` |
| anything else | page not found | `GoRouter::error_builder` |

## What to read

| File | What it shows |
|---|---|
| `src/common/routes.rs` | every path, route name and query key in one place; pages never write a path by hand |
| `src/common/routers.rs` | the routes, nested routes under `/`, reading `param`, `query` and `extra`, the error page, the start location from the command line, and the guard: a `redirect` that is a **plain function** (`guard_redirect`, unit-tested) plus `refresh_listenable` |
| `src/common/cart.rs` | the shared state: one `Cart`, made in `lib.rs`, handed to the router and from it to every page; `ValueNotifier`s, so a change rebuilds whoever reads it |
| `src/common/shell.rs` | the three sections: a bottom bar on a phone, a rail on a wider window (extended when there is room), a drawer of links, the cart's badge, the drawer's open flag kept with a hook |
| `src/common/shared.rs` | `page_app_bar`: a back button whenever the router can pop |
| `src/features/search/pages/searchpage.rs` | the address as the source of truth: the field writes `?q=`, the picker writes `?sort=`, and `did_update_widget` follows the address when it changes from outside |
| `src/features/product/pages/productpage.rs` | `push_named`, `push_with_extra`, `replace`, and an id in the address that names no product |
| `src/features/product/pages/sizepage.rs` | returning a result: write it to shared state, then `pop` |
| `src/features/checkout/pages/checkoutpage.rs` | `go_with_extra`, and emptying the cart while on a guarded page |

## The ideas

- **`go` says where; `push` says on top of what is here.** `go("/products/3")` makes the
  stack the routes that location is made of, so a nested page always has its parent beneath
  it, even when the app was opened at it. `push` adds one page and keeps the rest, which is
  how search stays beneath the product it opened. `replace` swaps the page on top, so going
  back skips it. `pop` removes the page on top.
- **Sections are top-level routes.** Switching with `go` makes the stack that one page, so
  the back button never walks through tabs.
- **The address is state.** The search query is in the location, not only in the page: a
  link, the web's back button or a reload shows the same search. A `go` to the same route
  with another query keeps the same page and its state, so the field keeps its focus.
- **Name the routes you link to.** `go_named("product", &[("id", "3")], &[])` writes the
  path and encodes parameters and query; `location_for` gives you the string when you need
  it (for `push_with_extra` or `replace`).
- **`extra` is a shortcut, not the truth.** It lives only in memory, so a page that reads
  it falls back to the address when it is missing (opened from a link, reloaded).
- **A result comes back through shared state.** The size page writes the size into the cart
  and pops; the product page, kept alive underneath, reads it when it is shown again.
- **Guards are redirects.** The router asks `guard_redirect` before every navigation, and
  again whenever `Cart::emptied` fires. The router listens only to what the guard depends on:
  whether the cart is empty, not every change of quantity.
- **Two kinds of "not found".** A location no route answers gets the router's
  `error_builder` page. A route that matched with an id naming nothing (`/products/abc`) is
  the page's to handle.
- **The back button comes from the router.** `page_app_bar` shows one whenever
  `router.can_pop()`. The app bar does not add it on its own: it only implies the drawer's
  menu button.

## Building against the published crate

Outside the frus checkout, replace the path dependency with `frus = "0.3"`.
