# Milestone 416 — The same bug, in the place I had not looked

Milestone 415 fixed one of the shell's two build paths and added a tripwire so that a third
one could not be added silently. The tripwire found the **second existing one** immediately —
the frame's own.

## What 415 missed

The frame path builds the tree and then, before any layout pass, counts its identities:

```rust
let tree = self.app.view(&theme);
let ids = collect_ids(tree.as_ref());
```

`collect_ids` walks `children()`. On a tree whose deferred subtrees have not been built, it
returns **one** identity — the root — and nothing else.

Everything downstream of `ids` is therefore blind to the whole application:

- **Mount fades.** `runtime.mounted.insert(id)` never sees a widget inside an `AppBar`, so
  nothing in a bar has ever faded in.
- **Leave fades.** `present` never contains them either, so nothing in a bar has ever faded
  out.
- And a widget that *moves* from outside a deferred subtree to inside one is in `mounted`
  from before and absent from `present` now — so it is snapshotted as leaving and a ghost is
  faded out over a widget that is still on the screen.

I wrote in 415 that the burst path was where the bug lived. It was where I had looked.

## The tripwire earned itself in a day

The `debug_assert` added in 415 would have panicked on the first frame of any debug build
with an `AppBar` — the demo included. That is unpleasant and it is exactly right: the guard
turned a silent, three-milestone-old behaviour gap into a crash that names its own fix, and
it did so before anyone ran the app.

## One way to build a tree

```rust
fn build_view<A: Application>(app: &A, theme: &Theme) -> Box<dyn Widget<A::Message>> {
    debug_assert!(MediaQuery::of().is_described(), "…install one first — milestone 416");
    let tree = app.view(theme);
    build_deferred(tree.as_ref(), theme);
    tree
}
```

Both sites call it. Neither of them was a mistake to be spotted at a call site — one reads
the tree with `collect_ids`, the other with `find_widget`, and both read it before the layout
pass would have prepared it. That is why it is one function and not two careful callers.

The surface assertion is the second half. A build outside a described surface measures text
at scale 1 while the frame lays it out at whatever the reader asked for — the mistake I made
inside milestone 415 itself, now caught rather than remembered.

## The first test that drives an application

The standing 🔴 says no test drives a whole frame with the shell's state around it, and this
milestone does not close it: `App<A>` holds an `EventLoopProxy` and an `Arc<Window>`, so it
cannot be constructed without an event loop.

But the part of the frame where two milestones' worth of bugs lived **can** be held on its
own, and now is. `frus-shell` has an `Application` implementation in its tests — an interface
that lives entirely inside a deferred subtree, which is what any application with an `AppBar`
is — and two assertions on it:

- a view built the shell's way has identities to mount, and every one of them can be found;
- the same view built the old way trips the wire.

Neither needs a window. It is the first test in this repository that drives an application
through the shell's own code, and the assertion in it is precisely the one that would have
caught both 415 and 416.

## What is still open

`App<A>`'s frame — events, focus, scroll, the whole state machine — remains untestable, and
the argument for extracting a windowless core is now two milestones old and two bugs long.
That stays 🔴.
