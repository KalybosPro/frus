# Milestone 303 — Async that is actually asynchronous

frus has had `Command::perform_async` since early on, and `async fn send` on the HTTP
client, and a `Future` in the type signature. Underneath, natively, every one of them
was this:

```rust
std::thread::spawn(move || {
    if let Some(message) = pollster::block_on(future) {
        let _ = proxy.send_event(message);
    }
});
```

A thread each, and `pollster::block_on`. Two things follow from that, and the second
is the one that matters.

## 1. A thread per effect

Ten concurrent requests were ten OS threads, each with a stack. Every subscription was
another thread, parked in `recv_timeout`. A screen polling three endpoints and running
two timers held five threads to do nothing but wait.

## 2. There was no reactor, so waiting did not work

`pollster::block_on` polls the future, and when it returns `Pending` it parks the
thread until the waker is called. **Nothing was there to call it.** A reactor is the
piece that watches timers and file descriptors and wakes the tasks whose turn has
come, and there wasn't one.

So a future that waited on anything outside itself waited for ever. What worked was:

- futures that are already ready, or that drive themselves;
- futures that **block internally** — which is why `fetch` had to use a blocking HTTP
  client, and why that was fine: it had a whole thread to waste.

Asynchrony existed in the type system and not underneath it. `Command::perform_async`
could not express *waiting*, which is the only thing async is for.

## What changed

One executor, on four worker threads, each running it **inside `async_io::block_on`**:

```rust
.spawn(move || async_io::block_on(shared.run(std::future::pending::<()>())))
```

That call is the whole milestone. `async_io::block_on` installs `async-io`'s reactor
on the thread; `futures_lite::future::block_on` does not. Swapping one for the other
compiles, passes any test that only awaits ready futures, and hangs the first time
anything waits on a timer. There is a test named after exactly that.

Deliberately **not tokio**. `async-executor` plus `async-io` is a scheduler and a
reactor, both small, pure Rust, no proc macros, and they work on Android — where the
framework has to run and where dragging in a general-purpose server runtime would be a
strange thing to ask of a UI application. The cost is stated plainly in the module
docs: a future that needs *tokio's* reactor will not run here, and such an application
should start its own runtime and hand messages back through `Command::run_async`.
Letting frus be **handed** a runtime instead of owning one is the obvious next step and
is not done.

## What it bought

| | before | after |
|---|---|---|
| 10 concurrent effects | 10 OS threads | 10 tasks on 4 threads |
| a subscription | a thread parked in `recv_timeout` | a task parked on the reactor |
| a future awaiting a timer | **hangs for ever** | wakes |

And a new effect that could not have existed before:

```rust
Command::after(Duration::from_secs(3), Msg::HideToast)
```

A real one-shot timer: a task on the reactor's timer wheel natively, a `setTimeout` on
the Web. A hundred pending timers are a hundred queue entries, not a hundred threads.
It is deliberately the *portable* primitive — `runtime::sleep` exists but is private,
because a native-only `sleep` in a cross-platform framework is an invitation to write
code that does not compile for one of its targets.

## The regression this would have been

Making the executor small makes **blocking on it** expensive, and the framework had a
blocking call sitting inside an `async fn`: `Request::send`, on `ureq`.

Under thread-per-future that was harmless — the thread was there to be blocked. On a
four-worker executor it means five slow requests stop *every other effect in the
application*, timers included. Moving to a shared executor without noticing this would
have made concurrency worse while claiming to make it better.

So the blocking call goes to a pool sized for blocking:

```rust
blocking::unblock(move || { /* ureq */ }).await
```

The general rule is now in `command.rs`'s own documentation, because it is the one
mistake this design invites: **something that blocks belongs in `Command::perform`**,
which gets a thread because a thread is what blocking needs. Something that *waits*
belongs in `perform_async`, because waiting no longer costs one.

## Also

`Command` grew a fourth kind of part, and `into_parts` returned a 3-tuple that would
have become a 4-tuple. It returns a named `Parts` struct instead: at a call site,
`parts.timers` says what it is and `.2` did not. Every constructor but `none()` now
writes only the field it fills and takes the rest from `..Self::none()`, so the fifth
kind of effect will not mean touching all seven of them again.

## Verification

- 974 workspace tests, up 6: four in `runtime`, two on `Command::after`.
  - `a_task_that_waits_on_a_timer_actually_wakes_up` — the reactor is installed. This
    is the test that would have failed against the old implementation, and against a
    plain `block_on` in the new one.
  - `more_waiting_tasks_than_there_are_threads` — 64 waiting tasks on 4 threads finish
    in the time of one wait, not sixty-four.
  - `dropping_a_handle_cancels_the_task` — how a subscription stops.
- `cargo check -p frus-shell --target wasm32-unknown-unknown`, with and without `net`.
  None of this exists on the Web: the browser is the executor, it always was, and the
  Web side of the framework was ahead of the native side rather than behind it.
- **On the device**, which is the check that mattered: `async-io` has to work on
  Android, and if it did not, the demo's stopwatch — a `Subscription::every`, now a
  task on the reactor rather than a thread parked in `recv_timeout` — would simply
  stop. From logcat on the Huawei STK-L21, running the release APK:

  ```
  18:00:30.749  [demo] stopwatch: 249s
  18:00:31.749  [demo] stopwatch: 250s
  18:00:32.749  [demo] stopwatch: 251s
  18:00:33.750  [demo] stopwatch: 252s
  ```

  One second apart to the millisecond, for as long as it was watched.
- fmt clean, clippy silent on both feature configurations.

## Left

- **Accept a runtime instead of owning one.** An `Executor` trait the application can
  implement, so an application already running tokio hands it over and the whole HTTP
  ecosystem becomes available. This is the piece that would make "async-first" true
  without qualification.
- **Subscriptions are not streams.** `Subscription` has exactly one kind, `Every`. Now
  that there is an executor, a subscription could be any `Stream<Item = Msg>` — a
  WebSocket, a file watcher, a channel — and the diff-by-id machinery is already there
  to start and stop them.
- `Command::perform` still gets a thread each, which is right for blocking work but
  unbounded; it should share the `blocking` pool.
