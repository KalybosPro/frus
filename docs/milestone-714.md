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

## Stopping a task, and function components

- **`TaskHandle<T>`.** `spawn_local`, `StateHandle::spawn` and `TaskScope::spawn` return one.
  Dropping it leaves the task running; `cancel()` drops the future at once, unfinished, and
  a task that cancels itself while it runs is not put back. Awaited from another task, it
  gives `Some(value)`, or `None` when the task ended without one — cancelled, or dropped with
  its owner. A guard inside the task's future marks the end and wakes whoever awaits, so the
  three ways to end are told the same way.
- **Function components.** `cx.task_scope()` gives a `TaskScope` whose tasks end with the
  component (they are alive while its hooks are not disposed), and `cx.async_callback(||
  async move { … })` is the handler built on it — the same shape as a state's, holding
  `UseState`s instead of the state's handle.
- **Every access to the task table goes through a function compiled once in frus**
  (`put`, `take`, `take_gone`, `#[inline(never)]`), as milestone 669 asks of every
  thread-local: a generic `spawn` inlined into an application would get a table of its own
  after a hot patch. The first push broke the guard test that says so; the local check ran
  only the library's tests, and now runs the crate's integration tests too.

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
- `local_task.rs`, for the handle: a cancelled task is dropped and never polled again;
  awaiting a handle gives what the task returned, and `None` for one dropped with its owner.
- `component.rs`: a task a function component owns ends when the component leaves the tree,
  and its handler changes the component's `UseState`.
- `relayout.rs`: a viewport sized by its content grows with it, and the same page is still
  reused.

## In a browser

`scripts/web-async-handler-check.py` drives the http example's form in headless Edge, through
the accessibility tree, and holds the POST in the browser (the DevTools protocol's Fetch
domain) to look at the page while the handler waits:

- while the request is held the page says "Sending…", and still does a second later: the
  handler changed the state before its `await`, and the frames go on while it waits;
- let go, the answer is shown and the form is unlocked;
- a second send is held and the page left while it waits: the answer that comes after shows
  nothing and throws nothing.

The first run failed at the answer: the card was cut off at the height the form had before.
Master did the same, with `use_future`: a pre-existing bug, not this milestone's. A
scrollable with no size of its own has been as tall as its content since milestone 590, but
the relayout cache still fingerprinted it as a leaf — style only — so a page that grew kept
its old viewport, frame after frame, until the window changed size. The test driver showed
it too once it was given the real page (the clip stayed at 352 px of 641). The fingerprint
now takes the content in whenever the viewport is measured from it; the rule that says when
(`viewport_sizing`) is shared by the layout and the fingerprint, so they cannot disagree.
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

- **A task replaced by the next press** is not cancelled for you: a handler that wants only the
  latest keeps the `TaskHandle` and cancels it (the guide's search example). A helper for that
  pattern can come when a second application asks for it.
- **A phone in a browser**: the check ran in headless Edge on the desktop.
