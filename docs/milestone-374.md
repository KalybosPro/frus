# Milestone 374 — An image from somewhere else

`Image::network("https://…/ada.png")`. The last of the reference's four image sources, and
the one that needed the two steps before it: bytes over HTTP from milestone 373, and a
process-wide store from 372.

## The widget cannot fetch, and should not

`frus-widgets` has no runtime, no socket, and no dependency on `frus-shell` — the
dependency runs the other way, and inverting it to reach a `fetch` would be a large price
for one call.

So the shell **registers** a fetcher on the way up, and the widget layer only asks. That is
the shape the decoder took a step earlier, for the same reason: the layer that knows *how*
is not the layer that knows *when*. An application embedding frus in a host that already
has an HTTP client points `set_image_fetcher` at that client instead.

A `fn` pointer rather than a boxed closure: it is registered once for the process, and a
plain function needs no allocation, no lifetime and no `Sync` wrapper to keep.

## Asked every frame, fetched once

`Image::network` is written in a `view`, and a view runs sixty times a second. The first
call starts the work and reports `Loading`; every later one is a lookup in the store.
Without that, one picture is sixty requests a second.

A failure is remembered for the reason 372 gave: a URL that 404s will 404 again, and
retrying every frame turns one bad link into a permanent load on somebody's server.

## Three states, and no builders

`Image::state()` returns `Ready`, `Loading` or `Failed(why)` — three because *not here
yet* and *will never be here* are different answers that an interface shows differently:
one is a placeholder, the other is a message.

There is deliberately no `loading` or `error` **slot**. The reference has them because its
load happens inside a stateful widget the application cannot see into. Here the state is
readable, so the application writes the branch it would have written anyway:

```rust
let photo = Image::network(&url);
match photo.state() {
    ImageState::Loading => CircularProgressIndicator::new().boxed(),
    ImageState::Failed(why) => text(why).boxed(),
    ImageState::Ready => photo.width(240.0).boxed(),
}
```

A closure stored in the widget would say the same thing somewhere the application cannot
see it, and would force `Image` to carry the message type for the sake of a branch the
view can already take. That is the framework's model rather than a gap in it — the same
call milestone 372 made about `error()`, and worth stating plainly rather than dressing up
as parity.

## Keeping the frames coming

An image in flight has to keep the interface drawing, or the frame that would show it
never happens.

It is a **count**, asked once when the interface is built, not a flag the walk reads off
the widget. Showing a placeholder means taking the image *out* of the tree — that is what
the `match` above does — and the fetch is still going on when the widget that started it is
gone. A hook on `Image` would go quiet at exactly the moment it is needed.

## Two bugs the tests found

**A tally beside the store can lie.** The count started as an `AtomicUsize`, incremented
when a fetch began and decremented in its callback. A test that left a request in flight
then poisoned every test after it — but the real fault is worse than test order: the tally
is a second copy of a fact the store already holds, and the two can disagree. It survives a
`forget_fetched_images` that empties the store, and a callback arriving after one
decrements it below zero, wrapping. Either way the interface is left redrawing for ever
over work that finished. Counting the `Loading` entries cannot drift, because there is
nothing to drift from.

**A request that never answers pins the screen.** In flight means redrawing, so a dead
server would leave an application repainting at full rate until someone closed it — on a
phone, on a battery. The fetcher carries a 30-second deadline: long enough for a photograph
on a slow connection, short enough that a hung socket becomes a failure the interface can
show and settle from. A request without an answer has to become one with a failure.

The synchronous test fetcher is the one worth keeping: it answers *inside* the call that
started it, so its callback locks the store while `fetched` is still running. That is the
harder case, and it is why the guard is dropped before the fetcher is called — held, it
would deadlock rather than fail.

## Left

`Image::file` — a path read at run time — is the fourth source, and the same store with a
path for a key. `repeat` and `filter_quality` still share one piece of work in `frus-gpu`.
A fetched image is never evicted: the store grows with the number of distinct URLs an
application shows, which is fine for a screen of avatars and wrong for an infinite feed.
That wants an LRU with a byte budget, which is the reference's `ImageCache` and its own
step.
