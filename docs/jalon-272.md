# Jalon 272 — `Request`: POST, headers and timeout on `fetch` (`net` feature)

## The goal

Milestone 271 shipped the foundation: `fetch(url)`, a cross-platform **text GET**. A real app needs
more — **posting** a body, **setting headers** (`Content-Type`, `Authorization`…), **bounding** the wait
with a timeout. This milestone adds a **request builder** covering all of that, without breaking the
`fetch` shorthand.

## The API

Two levels, one output signature (`Result<String, FetchError>`) for all three targets:

```rust
use frus::{Command, Request};
use std::time::Duration;

// The shorthand, unchanged: a text GET.
Msg::Load => Command::perform_async(async {
    match frus::fetch("https://example.com/api").await {
        Ok(body) => Msg::Loaded(body),
        Err(err) => Msg::Failed(err.to_string()),
    }
}),

// A JSON POST, a header, a deadline.
Msg::Save(json) => Command::perform_async(async move {
    let res = Request::post("https://example.com/api")
        .header("Content-Type", "application/json")
        .body(json)
        .timeout(Duration::from_secs(10))
        .send()
        .await;
    match res { Ok(_) => Msg::Saved, Err(e) => Msg::Failed(e.to_string()) }
}),
```

- `Request::{get, post, put, delete}(url)` or `Request::new(Method, url)`.
- `.header(name, value)` — **cumulative** (several calls overwrite nothing).
- `.body(text)` — the request's body (the last call wins).
- `.timeout(Duration)` — the deadline before giving up (returned as a `FetchError::Network`).
- `.send().await -> Result<String, FetchError>`.
- `fetch(url)` remains, and is exactly `Request::get(url).send().await`.

`Method`: `Get`, `Post`, `Put`, `Delete`, `Patch`, `Head` (`as_str()` → the HTTP verb).

## Implementation per platform

- **Native**: `ureq::request(method, url)`, `.set(name, value)` per header, `.timeout(dur)`, then
  `.send_string(body)` if a body is supplied, otherwise `.call()`.
- **Web**: `window.fetch` through a `web_sys::Request` built from a `RequestInit` (the method,
  `Headers`, the body). The **timeout** is armed by an `AbortController` whose signal is passed to the
  request; a `setTimeout` fires `abort()` past the deadline, and the timer is **disarmed**
  (`clearTimeout`) as soon as the response arrives.

The same chaining; the only difference is hidden behind two `#[cfg]`s.

## Verification

- **The native `--features net` build**: `frus-shell` and the `frus` facade compile (ureq + rustls).
- **The wasm `--features net` build** (`--target wasm32-unknown-unknown`): compiles — the
  `Request`/`RequestInit`/`Headers`/`AbortController`/`AbortSignal` bindings were added to the `web-sys`
  features.
- **Tests** (4): `error_display_is_readable`, `method_verbs`,
  `builder_accumulates_headers_body_and_timeout`, `fetch_shortcut_is_a_bare_get`. A real network round
  trip depends on the network/a browser — not run here; the transport is delegated to `ureq`/`web-sys`.

## What's left

- Streaming (an unbuffered body), binary responses, fine-grained redirects — as needed. The core (the
  method + headers + a body + a timeout) is there.
