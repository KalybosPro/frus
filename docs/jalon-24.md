# Jalon 24 — `Command` / effects from `update`

frus's Elm model gains its **effect channel**: `update` can now trigger work
outside the cycle (I/O, background task) whose result comes back as a message.
That was the missing piece for real applications.

## API

```rust
fn update(&mut self, msg: Msg) -> Command<Msg>;   // returns the effects
fn init(&mut self) -> Command<Msg> { Command::none() }  // start-up effect

Command::none()                          // no effect
Command::batch([a, b, c])                // several
Command::perform(|| compute() -> Msg)    // task → message fed back in
Command::run(|| { side_effect(); None }) // side effect, optional message
```

`Application::Message` is now `Clone + Send + 'static` (effects cross threads).

## Execution (framework)

- `run` opens the loop with **user events**:
  `EventLoop::<Message>::with_user_event()`, and keeps an
  `EventLoopProxy<Message>`.
- `update` returns a `Command`; the driver **spawns one thread per task**; on
  return, `proxy.send_event(msg)` **wakes the loop** → `user_event(msg)` →
  `dispatch(msg)` (which reapplies `update`, possibly producing further effects).
- `init()` runs once at start-up (inside `resumed`).

Every entry point (click, keyboard, drag, user event) goes through one central
`dispatch` that executes the returned `Command`.

## Technical decisions (alternatives)

- **Raw threads vs an async runtime (tokio/smol)** → **threads**: no heavy
  dependency, and enough for I/O and UI latency. An async executor stays a
  possible evolution (the tasks are already `FnOnce() -> Option<Msg> + Send`).
- **Returning results** through `EventLoopProxy` (winit's native mechanism for
  waking the loop from another thread) rather than a hand-rolled mpsc channel.

## Demo — task persistence

- **Save** → `Command::run` writes the tasks to a temporary file
  (`done<TAB>text`, **without serde**).
- **Load** / **start-up** → `Command::perform` reads the file →
  `Msg::Loaded(Vec<(bool, String)>)` replaces the tasks (ids reassigned).
- "Load" / "Save" buttons in the footer.

## Tests

- `Command`: `none`/`perform`/`run`/`batch` (structure + draining the tasks).
- Persistence: a `save → load` round trip (deterministic temporary file),
  `Msg::Loaded` replaces the tasks with unique ids, and `Save` does produce an
  effect (`!is_empty`) where a plain mutation produces none.
- Total: 35 frus-widgets + 4 frus-shell (Command) + 7 frus-demo + the doctest.

## Limits (v1)

- Raw threads: no cancellation and no pool; no `async`/`await`.
- Persistence in a hand-rolled text format (no migration or robustness), a single
  file.
- An immediate effect still makes a round trip through the loop (a slight delay).
