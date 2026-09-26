# Milestone 585 — Work that finishes later, in a component: `use_future`, `use_stream`, and the builders

A component could hold state, run effects and tick on a timer. It could not wait for
anything. A value that took time — a file read, a request, a computation — had to go through
the application's `update` as a `Command`. That is the Elm path, and it is not the component
model's. The reference's `FutureBuilder` and `StreamBuilder` are how an interface there shows
"loading, then the answer", and the audit after milestone 579 listed them with the other
builders.

## What it does

- **`cx.use_future(deps, || future)`** starts the future **once until `deps` change**. It
  returns an `AsyncSnapshot`: `Waiting` and no data at first, then `Done` with the value. A
  rebuild in between does not start it again. When `deps` change, a new future starts, and what
  the old one gives is not shown.
- **`cx.use_stream(deps, |sink| work)`** runs work that gives values as it goes, through
  `sink.add(value)`. The snapshot is `Waiting`, then `Active` with the latest value, then `Done`
  with the last one.
- The builders, under the reference's names, each a component:
  - **`FutureBuilder::new(key, make, build)`** and **`StreamBuilder::new(key, make, build)`**,
    over the two hooks;
  - **`ValueListenableBuilder::new(notifier, build)`**, which builds from a `ValueNotifier`'s
    value;
  - **`ListenableBuilder::new(listenable, build)`**, which builds from any notifier;
  - **`Builder::new(build)`**, a component under the reference's name.
- The reference's `StatefulBuilder` is not added. A component with `cx.use_state` is exactly
  that, and a second name for it would be a second thing to learn.

An error is a value like any other: work that can fail gives a `Result`, and the snapshot
carries it.

## How

- **The widget layer has no executor**, and no dependency that would bring one. The shell hands
  it one on the way up, `set_task_spawner`, as it already hands it the image fetcher: its own
  executor natively, the browser's `spawn_local` on the Web. With none registered, as in the
  widget crate's own tests or in a host that registers nothing, each piece of work gets a thread
  of its own, parked while it waits. On the Web it does not run.
- The future writes its value where the component reads it: a snapshot behind a mutex, kept in
  a memo keyed by `deps`, so it is started once. The work must be `Send` natively, because it
  runs on the executor's threads; on the Web nothing is (`MaybeSend`).
- **An arrival rebuilds.** Every value that arrives counts. Each shell keeps the count it last
  saw and rebuilds at the top of a frame when it has grown. A count and not a flag, so that one
  shell reading it takes nothing from another — in tests, where several shells run side by side
  in one process, a flag would be swallowed by whichever looked first. While any work is
  running, the shell keeps frames coming, as it does for an image being fetched, so that the
  arrival is seen.
- **A `ValueNotifier` needed nothing new.** It asks for a rebuild whenever it tells its
  listeners, so a component that reads it is always built from its present value. The two
  listenable builders are that, named.

## Checked against the reference

- The reference's snapshot has a connection state (`none`, `waiting`, `active`, `done`) and data
  or an error. `none` is not needed here: a snapshot exists only once the work is started.
- The error is carried in the data, as a `Result`.
- The reference's `StreamBuilder` takes a stream object. Here the stream is the work itself,
  giving values through a sink, which asks for no stream library.

## Verification

- Through the shell, with its real executor:
  - a future is `Waiting` at the first frame, then shows `done 42` once it comes, and is started
    once however many frames are built;
  - a new key starts a new future and shows its value;
  - a stream ends `Done` with its last value;
  - a `ValueNotifier` set to 5 shows 5 at the next frame.
- In the widget crate:
  - with no executor registered, the work runs on a thread of its own and its value arrives;
  - a snapshot starts waiting and empty.
- Mutations, each failing a test:
  - an arrival not rebuilding;
  - arrivals not counted;
  - a future never done;
  - a stream's value dropped;
  - a stream never done;
  - the key ignored;
  - no fallback thread;
  - the future restarted on every build.
- Not run on the Web or on a device: the Web path is a different executor (`spawn_local`) and
  has not been driven in a browser.

## Left

- **Frames while waiting.** While work runs the shell keeps frames coming, as it does for an
  image, so a slow request costs frames it does not need. Waking the loop from the executor
  when a value arrives would stop that. It needs a handle to the window in the spawner.
- **Cancellation.** Work whose key changed runs on to its end; only its value is ignored.
