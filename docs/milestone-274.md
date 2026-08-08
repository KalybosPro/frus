# Milestone 274 — `RemoteData<T, E>`: the Elm idiom for asynchronous data

## The goal

In milestone 273's example, the screen wrote an `Idle/Loading/Loaded/Failed` state machine **by hand**.
That is the pattern **every** network app rewrites — and often gets wrong (an ambiguous
`Option<Result<T, E>>`, or two `loading`/`error` booleans that can drift apart). This milestone ships
the established Elm idiom, [`RemoteData`], as a **framework type** (`frus::RemoteData`), then refactors
the example onto it.

## The type

```rust
pub enum RemoteData<T, E = String> {
    NotAsked,   // nothing requested yet (the initial state, Default)
    Loading,    // a request in flight
    Success(T), // the data arrived
    Failure(E), // the request failed
}
```

The four states are **exclusive**; matching on it in the `view` forces the compiler to handle each. `E`
defaults to `String` (the common case after a `FetchError::to_string()`).

**Methods**: `from_result(Result<T, E>)` (the bridge from an effect), `is_loading` / `is_success` /
`is_failure`, `value() -> Option<&T>`, `error() -> Option<&E>`, `as_ref() -> RemoteData<&T, &E>`
(matching without consuming), `map` / `map_err` (transforming a single case — e.g. decoding a body into
a domain type).

## Before / after (in `frus-fetch-example`)

```rust
// Before: a bespoke enum + two match arms in update.
enum Status { Idle, Loading, Loaded(String), Failed(String) }
Msg::Got(Ok(body)) => self.status = Status::Loaded(body.trim().to_string()),
Msg::Got(Err(err)) => self.status = Status::Failed(err),

// After: one framework type, one bridge.
joke: RemoteData<String>,
Msg::Got(res) => self.joke = RemoteData::from_result(res.map(|b| b.trim().to_string())),
```

The `view` matches `self.joke.as_ref()` across the four variants — no more ad-hoc type to maintain per
screen.

## Verification

- **6 tests** on `RemoteData`: `Default` = `NotAsked`, `from_result` (Ok/Err), the predicates +
  accessors, `map` (touching only `Success`), `map_err` (touching only `Failure`), `as_ref` (borrowing
  without moving).
- **`frus-fetch-example`** refactored: its 2 tests pass, the desktop **and** wasm builds OK.

## What's left

- A `view` helper that folds a `RemoteData` into widgets (a loading skeleton, a standard error panel) —
  as needed. The type itself is there and is enough to structure the state.
