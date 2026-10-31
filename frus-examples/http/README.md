# http

Talking to an HTTP API from a frus app — the same code on the desktop, Android and the Web.

The app is a client for the public test API at <https://jsonplaceholder.typicode.com>:

- **Posts** — `GET /posts`: a loading view, an error view with Retry, pull-to-refresh, and a
  refresh action in the app bar;
- **Post** (`/posts/:id`) — `GET /posts/{id}` and `GET /posts/{id}/comments` in parallel,
  then the author, `GET /users/{userId}`, once the post says who it is;
- **New post** (`/posts/new`) — a form that `POST`s JSON and shows what the server created,
  with a send in progress and its failure handled.

Every call has a 10-second timeout, and every failure is told in words the user can act on:
offline (airplane mode), too slow, not found, a server error.

## What it teaches

- **`frus::Request`** — the HTTP client: a method (`Request::get`, `Request::post`, …),
  headers, a body, a timeout, then `send()`. On the Web it is the browser's `fetch`;
  natively a blocking client run off the interface's thread. `frus::fetch(url)` is the
  one-line text GET for when nothing needs configuring.
- **Typed JSON** — `send_json::<T>()` decodes the response into a `serde` struct,
  `json_body(&value)` encodes one as the request body. Needs the `net` and `json` features.
- **Async work in a component** — `cx.use_future(deps, make)` runs a future once until
  `deps` change, and each build reads where it has got to; the framework rebuilds when the
  value arrives. Retry and refresh are a counter in the page's state, part of `deps`.
  `FutureBuilder` is the same thing as a widget, for a piece of a page that loads on its
  own.
- **An `async` handler** — the New post form's Send is `cx.async_callback(|state| async move
  { … })`: it locks the form, awaits the POST, and shows the answer, in that order and in one
  place. The task runs on the interface's thread, holds the page's state, and ends with the
  page if it is left before the answer comes.
- **`RemoteData`** — `Loading` / `Success` / `Failure`, the shape a page matches on.
- **An API client** — base URL, headers and timeout in one place, typed functions that
  return `Result<T, ApiError>`, and one mapping from transport errors to user-facing ones.

## Run it

```sh
cargo run                    # the list
cargo run -- /posts/1        # opens a post directly (the list is beneath it)
cargo run -- /posts/new      # opens the form
cargo apk run --lib          # Android
```

On Android the app must ask for the `INTERNET` permission — `Cargo.toml` does, under
`[[package.metadata.android.uses_permission]]`; without it every call fails as if the phone
were offline. On the Web the address bar plays the part of the command line.

```sh
cargo test                   # URLs, JSON decoding, error mapping, page state — no network
```

## Web

Once: `rustup target add wasm32-unknown-unknown` and
`cargo install wasm-bindgen-cli --version 0.2.127` (the version must match `wasm-bindgen` in
`Cargo.lock`). Then build, generate the JavaScript glue into `web/pkg/`, and serve `web/`:

```sh
cargo build --lib --target wasm32-unknown-unknown --profile web-release
wasm-bindgen --target web --no-typescript --out-dir web/pkg target/wasm32-unknown-unknown/web-release/http.wasm
python -m http.server 8080 --directory web     # http://localhost:8080
```

frus draws through WebGPU, which browsers offer on `https` pages and on `localhost`.

In the browser the requests are the browser's own, so the API must allow cross-origin
requests (CORS) — jsonplaceholder does, which is why this example uses it.


## The files

```
src/
  lib.rs                         module list + frus::main!(FrusApp::router(..))
  common/
    api.rs                       the client: BASE_URL, TIMEOUT, one async fn per call, ApiError
    remote.rs                    use_api: cx.use_future → RemoteData, keeping the old answer
                                 on screen while a reload is on its way
    routes.rs, routers.rs        the addresses, and the GoRouter (sub-routes under "/")
    shared.rs                    LoadingView, ErrorView (with Retry), the back button
  models/
    post.rs, comment.rs, user.rs the payloads, with serde (camelCase on the wire)
  features/
    posts/                       the list: RefreshIndicator over a virtualised ListView
    post/                        two requests in parallel; author_card is a FutureBuilder;
                                 the comments section has its own loading and error lines
    new_post/                    the form: validation, a POST the user starts, its status
```

Each feature has `pages/` (the `StatefulWidget` and its `build`), `states/` (the state as a
plain struct, unit-tested) and `widgets/` (pieces of the page).

## Things worth knowing

- **Name both features**: `frus = { version = "0.3", features = ["net", "json"] }`. With
  `json` alone, frus 0.3 turns the client on but does not export `Request` and `FetchError`.
- **A request runs once per `deps`**, not once per build. Never start a request from a
  handler by calling the API directly and dropping the future: change the state so the
  next build asks for it, as Retry, refresh and Send do here.
- **Rows of a `ListView` are plain widgets.** The list builds them during layout, after
  the framework has built the components; a `StatelessWidget` returned from the item
  builder is never built. `post_tile` is a function returning a `ListTile` for that reason.
- **The test server stores nothing.** A created post comes back with id 101, every time,
  and does not appear in the list.
