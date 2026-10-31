# Milestone 714 — A handler can await

## Objective

People trying frus 0.5 said that the framework must be asynchronous. It already had the
pieces: an executor with a reactor (milestone 303), `fetch` awaited on every platform,
`use_future`, `use_stream`, `FutureBuilder`, `StreamBuilder`, `spawn_async`. What it did not
have is the thing people write first: a handler that sends a form.

```rust
// What one wants to write: busy, the request, the answer, in order.
Button::new("Send").on_press(/* async: */ {
    state.sending = true;
    let answer = api::create_post(draft).await;
    state.sending = false;
    state.answer = Some(answer);
})
```

`spawn_async` runs its work on the shell's executor, off the interface's thread, so the work
must be `Send` and cannot hold the state's handle; the change waits in a second closure. The
http example did it the long way round: a `use_future` keyed on an `attempt` counter that only
Send changed, a `sending` field to carry the draft into the hook, and a `status(outcome)` that
read the hook's result back into the page's status. Three pieces of state to say one thing.

## What was done

- **Tasks on the interface's thread** (`frus-widgets/src/local_task.rs`). A future handed to
  `spawn_local` is kept on the interface's thread and polled there, between frames, when it is
  woken. It need not be `Send`: it may hold a `StateHandle`, a controller, an `Rc`. What it
  awaits waits elsewhere — a request on the shell's executor or the browser's loop — and its
  waker, called from whatever thread, marks the task and asks the shell for a frame.
- **The shell runs them at the start of a frame**, before it collects effects and builds: a
  task that changes a state asks for the rebuild that frame makes. The wake is the window's
  `request_redraw`, the one wake every platform honours (Android drops a bare one). The test
  driver's frames run them too, so an application's tests see what its handlers do.
- **Owned by a state.** `StateHandle::spawn(future)` ties a task to a state: when the state has
  left the tree, the task is dropped unfinished at the next frame, and nothing it would have
  changed is changed on a page that is gone. A free `spawn_local` task is owned by nothing.
- **`cx.async_callback(|state| async move { … })`**, on `StateContext` and `StateHandle`: a
  `Callback` that starts such a task each time it is called — what `on_press` takes.
- **A task never runs inside the handler that starts it**: its first poll is the next frame's.
  A handler returns before anything it started changes the state, and a task started from
  `init_state` runs once `init_state` is done.

## The http example, rewritten

The "New post" form now says what Send does in one place:

```rust
.on_press(cx.async_callback(|state| async move {
    state.set_state(|s| s.submitted = true);
    let Some(draft) = state.read(NewPostState::draft) else {
        return;
    };
    state.set_state(|s| s.status = SendStatus::Sending);
    let answer = api::create_post(draft).await;
    state.set_state(|s| s.finish(answer));
}))
```

The state lost `attempt`, `sending` and `status(outcome)`, and holds its `status` directly;
its tests are simpler for it (`draft()` and `finish(answer)`). The pages that load posts keep
`use_future`: data a page needs to show itself is what that hook is for.

## The guide

`docs/getting-started.md` gains **Work that waits**: which of the three shapes to use — an
`async` handler when the person starts it, `use_future` when the page needs it to show itself,
`spawn` when it works rather than waits — each with an example. The `async_callback` and
`StateHandle::spawn` examples in the API documentation are compiled as doctests.

## Tests

- `local_task.rs`: a task runs at the next frame, not in the handler; it goes on when what it
  awaits is given from another thread (and holds an `Rc` while it waits); a task waiting is not
  polled again until it is woken; a new task and a woken one ask for a frame; another thread's
  frame leaves this thread's wakes.
- `component.rs`: a task a state owns ends when the state leaves the tree (its future is
  dropped); an async handler changes the state before and after what it awaits, and each
  change asks for a rebuild.
- `widget_app.rs`, through the shell's driver: a tap shows "sending" at the next frame, and the
  answer another thread gives 20 ms later is shown in the frame after it came (20.7 ms, 8
  frames).
- `frus-examples/http`: the form's state tests, rewritten.
- Two doctests: `StateHandle::spawn`, `StateContext::async_callback`.

Wakes are tagged with the thread whose task they wake: an application has one interface
thread, but a test run has many, and one test's frame must not take another's wakes.

## Mutation testing

| Mutant | Verdict |
|---|---|
| A new task is not woken | killed (5 tests) |
| A gone state's task is kept | killed (`a_task_a_state_owns_ends_when_the_state_leaves_the_tree`) |
| A waiting task is dropped after its first poll | killed (3 tests) |
| A wake asks for no frame | killed (`a_woken_task_asks_for_a_frame`) |
| A frame takes every thread's wakes | survived, equivalent — see below |
| A frame removes every thread's wakes | killed (`another_threads_frame_leaves_this_threads_wakes`, added for it) |
| An owned task is owned by nothing | killed (`a_task_a_state_owns_ends_when_the_state_leaves_the_tree`) |
| The test driver's frames run no task | killed (`an_async_handler_shows_its_progress_and_its_answer`) |

The survivor reads another thread's wakes without removing them: task ids are unique across
threads, so an id a thread does not own finds nothing in its table and is skipped. What keeps
one thread's wakes from another is that only the thread's own are removed; the mutant on that
line is the one the new test kills.

## Not done

- **Function components** (`component(|cx| …)` with hooks) have no owned task: they use
  `spawn_local` with a `use_state` value, which is not dropped with the component.
- **Cancelling a task by hand**, or answering a value from one (`JoinHandle`). A task that is
  replaced — a second press while the first is out — runs beside it; the example's button is
  disabled while it sends.
- **A browser check.** On the web the same code runs on the browser's loop; it is compiled for
  wasm and linted there, not run in a browser for this milestone.
