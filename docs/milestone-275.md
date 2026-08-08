# Milestone 275 — Typed JSON on `Request` (`json` feature)

## The goal

`fetch` / `Request::send` return a `String`. A real app wants a **domain type** — `RemoteData<User>`,
not a `RemoteData<String>` it re-parses by hand on every screen. Behind a `json` feature, this milestone
adds both ends of the JSON bridge:

- **reading**: `Request::send_json::<T>()` deserialises the response into `T`;
- **writing**: `Request::json_body(&value)` posts a serialisable value.

## The API

```rust
use frus::{Request, RemoteData};

// Reading: a JSON response → a domain type.
#[derive(serde::Deserialize)]
struct User { id: u64, name: String }

let user: User = Request::get(url).send_json().await?;         // RemoteData<User> in the state

// Writing: a value → a JSON body (+ a Content-Type: application/json header).
#[derive(serde::Serialize)]
struct NewPost { title: String, body: String }

Request::post(url).json_body(&payload).send().await?;
```

- `send_json::<T: DeserializeOwned>() -> Result<T, FetchError>` = `send()` + `serde_json::from_str`; an
  unreadable body, or one that does not fit `T`, gives a `FetchError::Decode`.
- `json_body<B: Serialize>(&B)` serialises the body and sets the `Content-Type` header. The chaining
  stays **fluent**: a (rare) serialisation error is **deferred** and surfaces at `send()` (the builder
  pattern `reqwest` uses), through an `error: Option<FetchError>` field on `Request`.

## The feature

- `frus-shell`: `json = ["net", "dep:serde", "dep:serde_json"]` — **`json` implies `net`** (JSON only
  makes sense with the HTTP layer). `serde`/`serde_json` are **pure Rust**, so valid on all three
  targets (unlike `ureq`, native only).
- `frus` (the facade): `json = ["frus-shell/json"]`.
- By default `json` (like `net`) is **off**: no serde dependency embedded.

## Verification

- **Tests** (2 new, pure, no network): `json_body_serializes_and_sets_content_type` (a `{"x":1,"y":2}`
  body + the header set, no deferred error) and `decode_json_maps_valid_and_invalid_bodies` (valid
  parsing → the type; an unreadable body → `FetchError::Decode`). The decoding is isolated in a
  `decode_json` helper tested apart from the I/O.
- **Builds**: `--features json` (native + `wasm32-unknown-unknown`), the facade `--features json`, and
  the `net`-only / default combinations — all OK.

## What's left

- Non-2xx statuses with a JSON error body, an automatic `Accept: application/json` header — as needed.
  The bridge (reading/writing typed JSON) is there.
