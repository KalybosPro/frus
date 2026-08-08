# Milestone 273 — End-to-end network example (`frus-fetch-example`)

## The goal

Milestones 270–272 built the network stack (async effects, `fetch`, `Request` with
POST/headers/timeout) but **no screen exercised it**. This milestone ships the small missing example:
**loading an API and displaying it**, with the **loading → data → error** states.

It is also proof of the ergonomics we claim: **a single dependency** (`frus`, the `net` feature), **a
single entry point** (`frus::main!`), and the full Elm model.

## The screen

A button fires the request; the screen paints the current status:

```rust
enum Status { Idle, Loading, Loaded(String), Failed(String) }

fn update(&mut self, msg: Msg) -> Command<Msg> {
    match msg {
        Msg::Fetch => {
            self.status = Status::Loading;
            return Command::perform_async(async {
                let res = Request::get(JOKE_URL)
                    .header("Accept", "text/plain")
                    .timeout(Duration::from_secs(5))
                    .send()
                    .await;
                Msg::Got(res.map_err(|e| e.to_string()))
            });
        }
        Msg::Got(Ok(body)) => self.status = Status::Loaded(body.trim().to_string()),
        Msg::Got(Err(err)) => self.status = Status::Failed(err),
    }
    Command::none()
}
```

- **`update` stays pure**: the only impurity (the network) is confined to the `Command`; when the future
  resolves, the shell calls `update` back with `Got(...)`. Testable with no GPU.
- **`view` only paints the state**: a button + the `Status` rendering.

## The API queried

`https://icanhazdadjoke.com/` with the `Accept: text/plain` header — a joke in **plain text** (no JSON
to parse). The endpoint allows browser requests (**CORS**), so the example also works **on the Web**,
not just desktop/Android. The header + the timeout exercise milestone 272's `Request`.

## Verification

- **The desktop build**: `cargo build -p frus-fetch-example` — compiles.
- **The wasm build** (`--target wasm32-unknown-unknown`): compiles.
- **Tests** (2): `fetch_enters_loading_and_emits_an_effect` (switches to `Loading` **and** returns a
  non-empty effect), `result_messages_drive_the_state` (`Ok` → a trimmed `Loaded`, `Err` → `Failed`).
  The real network round trip is run by hand (`cargo run -p frus-fetch-example`) — not run here.

## Running it

```
cargo run -p frus-fetch-example
```
