# Milestone 270 — **Asynchronous** effects (`perform_async` / `run_async`)

## The goal

Until now, a `Command` only carried **synchronous** tasks (`FnOnce() -> Option<Msg>`): on the Web
(single-threaded), they ran as a microtask with no way to `await` — so **no real `fetch`**. This
milestone adds an **asynchronous** form: a `Command` can carry a **future** that genuinely awaits,
driven by the browser on the Web and run to completion on a thread natively.

## The API

- `Command::perform_async(future)` — the future's value becomes a message.
- `Command::run_async(future)` — a side-effecting future; `Option<Msg>` (`None` = no message).

```rust
fn update(&mut self, msg: Msg) -> Command<Msg> {
    match msg {
        Msg::Load => Command::perform_async(async {
            let body = fetch("/api/data").await;   // a real await (fetch on the Web)
            Msg::Loaded(body)
        }),
        Msg::Loaded(_) => Command::none(),
    }
}
```

## Execution per platform

- **Web** (`wasm32`): `wasm_bindgen_futures::spawn_local` drives the future — the browser is the
  reactor, a `fetch` (a `JsFuture`) awaits without blocking the loop. The message comes back through the
  proxy.
- **Native**: the future goes onto its **own thread** and is run to completion by `pollster::block_on`.
  Perfect for a **self-contained** future (a computation, a channel, a driven timer). **Real network
  I/O** (which needs a reactor) leans on the **application's async runtime** — the framework imposes no
  runtime.

### `Send` bounds per platform

The async task type is **conditional**: `Future + Send + 'static` natively (it crosses a thread),
`Future + 'static` on the Web (browser futures — `JsFuture` — are **not** `Send`, and do not need to be
when single-threaded). So both `perform_async` / `run_async` signatures are `#[cfg]`-guarded.

## Implementation

- **`frus-shell/src/command.rs`**: the `async_tasks: Vec<AsyncTask<Msg>>` field (a `#[cfg]`-guarded
  alias), the `perform_async` / `run_async` methods (two variants per platform), `batch` / `is_empty` /
  `into_parts` extended.
- **`frus-shell/src/app.rs`** (`run_command`): drains `async_tasks` — `thread::spawn` +
  `pollster::block_on` natively, `spawn_local` on the Web; the message returned through the proxy, like
  the synchronous tasks.

## Verification

- **Compilation**: `frus-shell` compiles (tests included, `--no-run`).
- **Tests** (native): `perform_async_yields_a_message` (`block_on(async { 7 })` → `Some(7)`),
  `run_async_may_produce_nothing`, `batch_combines_sync_and_async_tasks`.
  *(Running the test binaries locally is blocked by SAC this session — os error 4551, an environment
  issue; compilation itself passes. See the project's SAC note.)*
- **Web**: the `spawn_local` path is structurally identical to the old one (already in place in
  milestone 130); verifiable in a browser.

## What's left

- A **cross-platform `fetch` helper** (web-sys `fetch` ↔ a native client) — for now, the app supplies
  the future; the framework only drives it.
