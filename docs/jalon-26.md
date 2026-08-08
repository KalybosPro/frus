# Jalon 26 — Subscriptions (continuous message sources)

The last big piece of the Elm model: the **stream** counterpart to `Command`
(which is one-shot). The app declares **continuous sources** of messages
according to its state; the framework **starts and stops** them by diffing.

## API

```rust
fn subscription(&self) -> Subscription<Msg> { Subscription::none() }  // added to the trait

Subscription::none()
Subscription::batch([...])
Subscription::every(Duration, |instant| Msg)   // one message per interval
```

Each subscription carries a stable **id** = a hash of its recipe (kind +
duration). Two `every(1s)` = **a single** subscription.

## How it works (framework)

- `sync_subscriptions()` is called **at start-up** and after **every**
  `dispatch`: it compares the declared ids with the running ones
  (`HashMap<u64, Sender<()>>`) → starts the new ones, **cancels** the ones that
  have gone.
- One `every` = one thread looping on `rx.recv_timeout(interval)`:
  - **Timeout** → `proxy.send_event(make(now))` (the message comes back into the
    loop through `user_event`, like a `Command` result);
  - **Sender dropped** (cancellation) or the loop closed → the thread exits.
- Cancelling = removing the `Sender` from the table (dropping it); the thread
  exits at its next wake-up. Complete symmetry with `Command` (same proxy, same
  threads).

## Technical decisions (alternatives)

- **One thread per subscription** (consistent with `Command`) vs a single *timer
  wheel* → one thread per sub, simple and sufficient.
- **Diffing by recipe hash** (the Elm way) → two identical `every`s merge; a
  subscription persists for as long as it is redeclared identically.

## Demo — a stopwatch

In the header: "· Ns" (elapsed seconds) + a **Pause/Resume** button. `running`
drives the subscription: `running ? every(1s, |_| Tick) : none()`. Toggling it
demonstrates the thread genuinely **starting and stopping** through the diff.
`init()` starts the stopwatch; `Msg::Tick` increments the counter.

## Tests

- `Subscription`: `none`/`is_empty`, `every` → a stable id per duration (same
  duration = same id, different durations = different ids), `batch` combines.
- Demo: `subscription()` empty when paused, non-empty otherwise (stable id across
  two evaluations); `Tick` increments the counter.
- **End to end**: the demo ran for 6 s → **5 ticks** observed in the logs
  (1s→5s), proving the `every → proxy → user_event → update` stream works.
- Totals: 3 frus-shell (subscription) + 9 frus-demo.

## Limits (v1)

- One thread per subscription (no pool); cancellation latency ≤ one interval.
- Only `every` for now (the mechanism will accept further `Kind`s: global
  keyboard, window events, external streams).
- A formatted wall clock would need a time library → an **elapsed** stopwatch
  instead.
