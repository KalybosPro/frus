# Milestone 130 — Effects & subscriptions on the Web

## Analysis

Milestone 129 got the whole stack **compiling** for `wasm32-unknown-unknown` + WebGPU,
but the framework layer was still bound to native platforms on one point: **effects**
(`Command`) and **subscriptions** (`Subscription::every`) ran on `std::thread::spawn`.
wasm32 is **single-threaded** — a `thread::spawn` there panics. As a result, on the Web
a purely input-driven app (the button counter) ran, but any app with an effect or a
subscription-driven animation would have collapsed at the first `update` returning a
`Command`, or at the first active subscription.

The goal: port both mechanisms to the Web **without touching the application API** —
`Command::perform`/`run` and `Subscription::every` stay identical; only their execution
differs per platform.

## Technical decisions

- **Effects → `spawn_local`.** A `Command` is a list of synchronous tasks
  (`FnOnce() -> Option<Msg>`). On the Web, each task is scheduled onto the loop through
  `wasm_bindgen_futures::spawn_local` (a microtask) instead of a thread; its message
  comes back through the same `EventLoopProxy` as natively. The work stays **synchronous**
  (the `Task` type cannot `await`) — enough for a compute effect; a genuinely
  asynchronous effect (a network fetch) will later need a dedicated `Command` variant.

- **Subscriptions → `setInterval`.** The `every` subscription ran in a thread looping on
  `recv_timeout`. On the Web, it becomes a **browser `setInterval`** (`web-sys`): on each
  tick, the callback emits the message through the proxy. **Cancellation** — the
  cornerstone of subscription diffing — is preserved by a `web_timer::Interval` handle
  whose **drop** calls `clearInterval` (and releases the retained closure). An exact
  mirror of native, where dropping the `Sender` makes the thread exit.

- **`SubHandle`, one handle per platform.** `running_subs` now maps each id to a
  `SubHandle`: `Sender<()>` natively, `Option<web_timer::Interval>` on the Web. The diff
  (`retain` + `insert`) and stop-by-drop stay identical; all the `sync_subscriptions`
  logic is unchanged.

- **`web-time` for the tick clock.** The `Instant` passed to the message factory
  (`make(Instant::now())`) comes from `web-time` — in place since J129 — so it is valid
  on all three platforms.

- **Showcase: auto mode in `frus-hello`.** The counter gains a **Start/Stop auto**
  button: in auto mode, `subscription()` returns `every(1s, |_| Tick)`; otherwise
  `none()`. It is the smallest example that makes a subscription **visible** in a
  browser — and it tests without a GPU (subscription diffing is pure).

## Implementation

- `frus-shell/src/app.rs`: the `web_timer` module (Web) — a retained `Interval`,
  `clearInterval` on drop; a per-platform `SubHandle` type; `run_command` and
  `start_subscription` split native/Web (`spawn_local` / `setInterval`); the
  `Sender`/`RecvTimeoutError` import restricted to native.
- `frus-shell/src/reload.rs`: `restore_from_env` (and the `Path` import) restricted to
  their only caller, the desktop `run` — removing dead code on Android/Web.
- `frus-hello/src/lib.rs`: an `auto` state, the `ToggleAuto`/`Tick` messages,
  `subscription()` following the state, the toggle button, the
  `auto_mode_drives_the_subscription` test.

## Verification

- **Compiles** for `wasm32-unknown-unknown` (effects + subscriptions included), **with no
  warning**.
- **Native intact**: `cargo test --workspace` stays **green** (no regression), including
  the new auto-mode test.
- The subscription is **tested purely**: absent at rest, present once `auto` is on, a
  `Tick` increments, gone once switched off.

## What's left

- **Verification in a real browser** (the *seeing* step): Start auto → the counter
  increments once a second, Stop → it stops. I cannot launch a browser here.
- **A genuinely asynchronous effect on the Web** (a network fetch): will need a `Command`
  variant able to `await` (the current `Task` is synchronous).
- Clipboard / IME / accessibility on the Web remain separate pieces of work.
