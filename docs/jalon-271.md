# Jalon 271 — Cross-platform `fetch` helper (`net` feature)

## The goal

Milestone 270 gave us the **mechanism** for an asynchronous effect (`Command::perform_async`) but left
the app to supply the future — so to touch `web-sys` / an HTTP client itself. This milestone ships the
**missing helper**: a **cross-platform** HTTP GET, `frus::fetch(url).await`, one signature for all three
targets.

## The API

```rust
use frus::{Command, fetch};

Msg::Load => Command::perform_async(async {
    match fetch("https://example.com/api").await {
        Ok(body) => Msg::Loaded(body),
        Err(err) => Msg::Failed(err.to_string()),
    }
}),
```

- `async fn fetch(url: impl Into<String>) -> Result<String, FetchError>` — a GET, the body as text.
- `FetchError`: `Network(String)` (transport/DNS/TLS), `Status(u16)` (non-2xx), `Decode(String)` (an
  unreadable body). It implements `Display` + `Error`.

## Implementation per platform

- **Web** (`wasm32`): `window.fetch` through `web-sys` (+ the `Response` feature), a real `await` — the
  future is **not** `Send`, which the Web's `perform_async` tolerates.
- **Native**: the blocking **`ureq`** client (rustls TLS included), executed **inside the future's
  body** — run to completion on `perform_async`'s dedicated thread, where blocking is safe. The future
  stays `Send`.

The same signature; the only difference is hidden behind two `#[cfg]`s.

## Behind a feature (opt-in)

- **`frus-shell`**: `[features] net = ["dep:ureq"]`; `ureq` is an **optional native** dependency; the
  `net` module and the re-exports (`fetch`, `FetchError`) are `#[cfg(feature = "net")]`-guarded.
- **`frus`** (the facade): `[features] net = ["frus-shell/net"]` + the
  `frus::{fetch, net, FetchError}` re-export.
- **`net` is off by default**: an app that does no networking embeds **neither `ureq` nor its TLS
  stack**. You turn it on with `frus = { path = "…", features = ["net"] }`.

## Verification

- **The default build** (`net` off): `frus-shell` compiles, unchanged — no cost.
- **The `--features net` build**: `frus-shell` compiles with `ureq` + rustls.
- **Test**: `error_display_is_readable` (the `FetchError` formatting). A real GET depends on the
  network/a browser — not run here (and the test binaries are blocked by SAC this session); the
  transport logic is delegated to `ureq`/`web-sys`.

## What's left

- Headers, **POST**/bodies, timeouts, streaming — as needed. The foundation (a GET `fetch`) is there.
