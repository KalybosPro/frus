# Milestone 551 — Components: stateless, stateful, hooks

## Objective

An application was a message type, an `update` and a `view`: every interaction became a
value, the value went to one function, and that function changed one struct. It is a
sound model and a demanding one — a screen with a counter needs an enum, a match arm and
a dispatch, and a widget that keeps something to itself (a hover, a draft, a timer) has
no place to keep it but the one struct.

The objective is the model most interfaces are actually written in: **a widget is a
small object that says what it looks like; one that has something to remember has a
state that outlives the rebuilds; an interaction runs a closure.**

## What was added

- **`StatelessWidget`** — `build(&self, cx: &BuildContext) -> Box<dyn Widget>`. Any
  `Fn(&BuildContext) -> Box<dyn Widget>` is one, which is how a function becomes a
  component.
- **`StatefulWidget` and `State`** — the widget is configuration (made again on every
  rebuild); its `State` is what stays. `init_state`, `did_update_widget`, `dispose` and
  `build`, with a `StateContext` that carries the widget, a `StateHandle` and a
  `callback(|state| …)` shorthand for a handler that calls `set_state`.
- **Hooks** on `BuildContext` — `use_state`, `use_ref`, `use_memo`, `use_effect` — so a
  function component keeps state without a struct. Called in the same order every time; a
  different order is caught, and says so.
- **`Callback`** — what a handler is once a widget holds it.
- **`FrusApp`** — an application whose whole declaration is a root component (`from_fn`,
  `new`, `stateful`) and how it is dressed (title, themes, size, density). It implements
  `Application`, so the shell drives it through the contract it always had.

## Decisions

**The tree is still rebuilt from scratch.** `set_state` does not patch anything; it asks
for a rebuild, and every component builds again. That is what the framework already did on
every message, and it is what keeps identity simple. What is new is that a component's
state is *kept* across that rebuild.

**A state is found by where its widget sits, or by its key.** The runtime holds a store
keyed by the widget's position in the tree (the identity it already gives every node) plus
how deep a chain of components that position is — a component that builds another
component directly is the same place as it. A key (`keyed(..)` around the component, or
`Component::with_key`) moves the identity off the position, so a state follows its item
when siblings are reordered. A different kind of widget in the same place replaces the
state and disposes the old one.

**Disposal is sweep-at-end-of-frame.** Each rebuild bumps an epoch; every state a build
reaches is stamped with it; when the frame that laid the tree out is done, what was not
stamped is disposed. Effects run there too, after the tree is built, so that what an effect
does — start a timer, ask for another frame — never happens in the middle of building.

**A component is transparent.** Once built it *is* what it built: same box, same children,
same behaviour, through the same forwarding macro `Keyed` and `Themed` use. Building has to
happen *before* the walk asks a node about its theme scope, its surface or its children —
those answers are the built widget's — so a new `Widget::expand` hook runs first in the
two walks, and every transparent wrapper forwards it. A component's *key* is its own and
never its child's: a parent reads it to place the component before the component is built,
and an answer that changed when the child arrived would move the place under the state.

**`Callback` is a handle, not a closure.** The first version held an `Rc<dyn Fn()>` and
the shell refused it: an application's message must be `Send`, because effects and
subscriptions move messages between threads, and a closure over a state handle is not. The
alternatives were to relax that bound — which meant splitting `Command`, `Subscription`
and the effect runner over two kinds of application, in the busiest file of the shell — or
to make the message a handle. It is a small `Send` handle to a closure held in a table on
the thread that made it; the last handle to go releases the closure; running one anywhere
else finds nothing (and says so in a debug build). Nothing unsound is reachable.

**The default message type is `Callback`.** Every widget type now reads
`Button<Msg = Callback>`, so an application says `Box<dyn Widget>` and `Button` and
never mentions a message. An application that wants typed messages still has them: the
old `Application` model is untouched underneath, and every existing test and screen runs
as before.

**A handler can just be a closure.** `on_press(|| …)` — the by-value setters take
`impl Into<Msg>`, and a closure converts into a `Callback`. The value setters
(`on_toggle(|on| …)`, `on_change(|v| …)`) take a closure returning *anything that is a
message*, or nothing: an `IntoMsg` trait turns `()` into a message that runs the work.
The work is **deferred to delivery**, not done when the message is made. A first version
ran it at once and a test caught the reason: a checkbox asks for its message more than
once for a single tap, so the handler ran twice.

## Cost

Three places where an `Msg`-typed test needed a type written that a value used to
imply (`Button::<()>`, `7usize`, `-> Msg { unreachable!() }`): inference no longer learns
the message type from the value passed. No behaviour changed for an existing application.

## Verification

- 12 tests on the store and the lifecycle: a state is made once and survives rebuilds; it
  is told what changed; it is disposed when a build no longer reaches it; a different kind in
  the same place starts over; a key carries it through a reorder; nested components keep
  both states; hooks keep values, `use_memo` recomputes only on change, effects run after the
  build, clean up before the next run and at the end; a different hook order is refused;
  `set_state` from inside `build` is refused.
- 7 tests on `Callback`: identity, conversion, `Send`, release of the closure with the last
  handle, release from another thread, a callback that owns callbacks.
- 4 end-to-end tests through the shell's windowless driver: a hook survives three taps and the
  tree is built again with the new value; a stateful root keeps its state between taps; a
  value handler that returns nothing runs **once** per tap; an effect's change is shown on
  the next frame.
- `frus-widgets` (1660 tests, 61 doctests), the workspace check and clippy with
  `-D warnings`.

## Left out

Text controllers and the router — the next two milestones. A shared, subtree-scoped
inherited value (only application-wide services exist), and fine-grained rebuilds: the tree
still rebuilds as a whole.
